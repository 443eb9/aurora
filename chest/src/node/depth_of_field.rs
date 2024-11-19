use aurora_core::render::{
    flow::{RenderContext, RenderNode},
    resource::DynamicGpuBuffer,
    scene::GpuScene,
};
use encase::ShaderType;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BufferBindingType, BufferUsages, Color,
    ColorTargetState, ColorWrites, FilterMode, FragmentState, LoadOp, Operations,
    PipelineLayoutDescriptor, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
    StoreOp, Texture, TextureDescriptor, TextureSampleType, TextureView, TextureViewDimension,
    VertexState,
};

use crate::node::DEPTH_PREPASS_TEXTURE;

pub enum DofPass {
    GaussianHorizontal,
    GaussianVertical,
    HexagonVertAndDiag,
    HexagonRhomboid,
}

pub struct GaussianDof {
    pub horizontal: RenderPipeline,
    pub vertical: RenderPipeline,

    pub layout: BindGroupLayout,
    pub config: DynamicGpuBuffer,
    pub sampler: Sampler,
}

pub struct HexagonDofTargets {
    pub target_a: Texture,
    pub target_view_a: TextureView,
    pub target_b: Texture,
    pub target_view_b: TextureView,
}

pub struct HexagonDof {
    pub vert_and_diag: RenderPipeline,
    pub vert_and_diag_layout: BindGroupLayout,
    pub rhomboid: RenderPipeline,
    pub rhomboid_layout: BindGroupLayout,

    pub mrt: HexagonDofTargets,
    pub config: DynamicGpuBuffer,
    pub sampler: Sampler,
}

#[derive(ShaderType)]
pub struct DepthOfField {
    pub focal_length: f32,
    pub focal_distance: f32,
    pub coc_factor: f32,
    pub max_coc_radius: f32,
    pub max_depth: f32,
}

impl Default for DepthOfField {
    fn default() -> Self {
        Self {
            focal_length: 4.0,
            focal_distance: 20.0,
            coc_factor: 1.0,
            max_coc_radius: 50.0,
            max_depth: 30.0,
        }
    }
}

pub enum DepthOfFieldData {
    Gaussian(GaussianDof),
    Hexagon(HexagonDof),
}

#[derive(Default)]
pub enum DepthOfFieldMode {
    Gaussian,
    #[default]
    Hexagon,
}

#[derive(Default)]
pub struct DepthOfFieldNode {
    pub config: DepthOfField,
    pub mode: DepthOfFieldMode,

    pub data: Option<DepthOfFieldData>,
}

impl DepthOfFieldNode {
    pub fn draw_gaussian(
        &self,
        GpuScene { assets, .. }: &mut GpuScene,
        RenderContext {
            device,
            queue,
            targets,
            ..
        }: &RenderContext,
        data: &GaussianDof,
        pass_type: DofPass,
    ) {
        let post_processing = targets.swap_chain.start_post_process();

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("dof_gaussian_bind_group"),
            layout: &data.layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &assets.texture_views[&DEPTH_PREPASS_TEXTURE.view],
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(post_processing.src),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&data.sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: data.config.entire_binding().unwrap(),
                },
            ],
        });

        let mut command_encoder = device.create_command_encoder(&Default::default());

        {
            let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("dof_gaussian_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: post_processing.dst,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(match pass_type {
                DofPass::GaussianHorizontal => &data.horizontal,
                DofPass::GaussianVertical => &data.vertical,
                _ => unreachable!(),
            });
            pass.set_bind_group(0, assets.common_bind_group.as_ref().unwrap(), &[]);
            pass.set_bind_group(1, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit([command_encoder.finish()]);
    }

    pub fn draw_hexagon(
        &self,
        GpuScene { assets, .. }: &mut GpuScene,
        RenderContext { device, queue, .. }: &RenderContext,
        data: &HexagonDof,
        pass_type: DofPass,
        color_attachments: &[Option<RenderPassColorAttachment>],
        color_src0: &TextureView,
        color_src1: &TextureView,
    ) {
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("dof_hexagon_vert_and_diag"),
            layout: &data.vert_and_diag_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &assets.texture_views[&DEPTH_PREPASS_TEXTURE.view],
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(color_src0),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&data.sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: data.config.entire_binding().unwrap(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(color_src1),
                },
            ],
        });

        let pipeline = match pass_type {
            DofPass::HexagonVertAndDiag => &data.vert_and_diag,
            DofPass::HexagonRhomboid => &data.rhomboid,
            _ => unreachable!(),
        };

        let mut command_encoder = device.create_command_encoder(&Default::default());

        {
            let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("dof_hexagon_pass"),
                color_attachments,
                ..Default::default()
            });

            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, assets.common_bind_group.as_ref().unwrap(), &[]);
            pass.set_bind_group(1, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit([command_encoder.finish()]);
    }
}

impl RenderNode for DepthOfFieldNode {
    fn require_shaders(&self) -> Option<&'static [(&'static [&'static str], &'static str)]> {
        Some(&[
            (&[], include_str!("../shader/fullscreen.wgsl")),
            (
                &[
                    include_str!("../shader/common/common_type.wgsl"),
                    include_str!("../shader/common/common_binding.wgsl"),
                    include_str!("../shader/fullscreen.wgsl"),
                    include_str!("../shader/math.wgsl"),
                ],
                include_str!("../shader/post_processing/depth_of_field.wgsl"),
            ),
        ])
    }

    fn build(
        &mut self,
        GpuScene { assets, .. }: &mut GpuScene,
        RenderContext {
            device,
            queue,
            node,
            targets,
        }: RenderContext,
    ) {
        let mut bf_config = DynamicGpuBuffer::new(BufferUsages::UNIFORM);
        bf_config.push(&self.config);
        bf_config.write::<DepthOfField>(device, queue);

        let mut entries = vec![
            // Depth
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Depth,
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // Color
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            // Sampler
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
            // Config
            BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(DepthOfField::min_size()),
                },
                count: None,
            },
        ];

        if matches!(self.mode, DepthOfFieldMode::Hexagon) {
            entries.push(BindGroupLayoutEntry {
                binding: 4,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
        }

        let common_layout = assets.common_layout.as_ref().unwrap();
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("dof_layout"),
            entries: &entries,
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("dof_pipeline_layout"),
            bind_group_layouts: &[common_layout, &layout],
            ..Default::default()
        });

        let target_states = &[Some(ColorTargetState {
            format: targets.color_format,
            blend: None,
            write_mask: ColorWrites::ALL,
        })];
        let mut desc = RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &node.shaders[0],
                entry_point: "vertex",
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &node.shaders[1],
                entry_point: "",
                compilation_options: Default::default(),
                targets: target_states,
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: Default::default(),
            cache: None,
        };

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("dof_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        match self.mode {
            DepthOfFieldMode::Gaussian => {
                desc.label = Some("dof_gaussian_horizontal");
                desc.fragment = desc.fragment.map(|f| FragmentState {
                    entry_point: "gaussian_horizontal",
                    ..f
                });
                let horizontal = device.create_render_pipeline(&desc);

                desc.label = Some("dof_gaussian_vertical");
                desc.fragment = desc.fragment.map(|f| FragmentState {
                    entry_point: "gaussian_vertical",
                    ..f
                });
                let vertical = device.create_render_pipeline(&desc);

                self.data = Some(DepthOfFieldData::Gaussian(GaussianDof {
                    horizontal,
                    vertical,
                    layout,
                    config: bf_config,
                    sampler,
                }));
            }
            DepthOfFieldMode::Hexagon => {
                let vert_and_diag_layout =
                    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                        label: None,
                        entries: &entries,
                    });
                let rhomboid_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
                    entries: &entries,
                });

                let target_states = [target_states[0].clone(), target_states[0].clone()];
                let vert_and_diag = device.create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some("dof_hexagon_vert_and_diag"),
                    fragment: desc.fragment.clone().map(|f| FragmentState {
                        entry_point: "blur_vert_and_diag",
                        targets: &target_states,
                        ..f
                    }),
                    ..desc.clone()
                });

                let rhomboid = device.create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some("dof_hexagon_rhomboid"),
                    layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                        bind_group_layouts: &[common_layout, &rhomboid_layout],
                        ..Default::default()
                    })),
                    fragment: desc.fragment.clone().map(|f| FragmentState {
                        entry_point: "blur_rhomboid",
                        ..f
                    }),
                    ..desc.clone()
                });

                let tex = targets.swap_chain.desc();
                let target_a = device.create_texture(&TextureDescriptor {
                    label: Some("dof_mrt_a"),
                    ..tex.clone()
                });
                let target_b = device.create_texture(&TextureDescriptor {
                    label: Some("dof_mrt_b"),
                    ..tex.clone()
                });

                self.data = Some(DepthOfFieldData::Hexagon(HexagonDof {
                    vert_and_diag,
                    rhomboid,
                    vert_and_diag_layout,
                    rhomboid_layout,

                    mrt: HexagonDofTargets {
                        target_view_a: target_a.create_view(&Default::default()),
                        target_a,
                        target_view_b: target_b.create_view(&Default::default()),
                        target_b,
                    },
                    config: bf_config,
                    sampler,
                }));
            }
        }
    }

    fn draw(&self, scene: &mut GpuScene, context: RenderContext) {
        match self.data.as_ref().unwrap() {
            DepthOfFieldData::Gaussian(data) => {
                self.draw_gaussian(scene, &context, data, DofPass::GaussianHorizontal);
                self.draw_gaussian(scene, &context, data, DofPass::GaussianVertical);
            }
            DepthOfFieldData::Hexagon(data) => {
                let post_process = context.targets.swap_chain.start_post_process();
                self.draw_hexagon(
                    scene,
                    &context,
                    data,
                    DofPass::HexagonVertAndDiag,
                    &[
                        Some(RenderPassColorAttachment {
                            view: &data.mrt.target_view_a,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(Color::TRANSPARENT),
                                store: StoreOp::Store,
                            },
                        }),
                        Some(RenderPassColorAttachment {
                            view: &data.mrt.target_view_b,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(Color::TRANSPARENT),
                                store: StoreOp::Store,
                            },
                        }),
                    ],
                    post_process.src,
                    post_process.dst,
                );
                self.draw_hexagon(
                    scene,
                    &context,
                    data,
                    DofPass::HexagonRhomboid,
                    &[Some(RenderPassColorAttachment {
                        view: post_process.dst,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::TRANSPARENT),
                            store: StoreOp::Store,
                        },
                    })],
                    &data.mrt.target_view_a,
                    &data.mrt.target_view_b,
                );
            }
        }
    }
}
