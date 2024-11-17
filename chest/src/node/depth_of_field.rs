use aurora_core::render::{
    flow::{RenderContext, RenderNode},
    resource::DynamicGpuBuffer,
    scene::GpuScene,
};
use encase::ShaderType;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BufferBindingType, BufferUsages, Color,
    ColorTargetState, ColorWrites, ComputePipeline, Extent3d, FilterMode, FragmentState, LoadOp,
    Operations, PipelineLayoutDescriptor, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor,
    ShaderStages, StoreOp, Texture, TextureDescriptor, TextureDimension, TextureSampleType,
    TextureUsages, TextureViewDimension, VertexState,
};

pub struct GaussianDof {
    pub horizontal: RenderPipeline,
    pub vertical: RenderPipeline,

    pub layout: BindGroupLayout,
    pub config: DynamicGpuBuffer,
    pub sampler: Sampler,
}

#[derive(ShaderType)]
pub struct DepthOfField {
    pub aperture_diameter: f32,
    pub focal_length: f32,
    pub coc_factor: f32,
}

impl Default for DepthOfField {
    fn default() -> Self {
        Self {
            aperture_diameter: Default::default(),
            focal_length: Default::default(),
            coc_factor: Default::default(),
        }
    }
}

pub enum DepthOfFieldData {
    Gaussian(GaussianDof),
}

#[derive(Default)]
pub enum DepthOfFieldMode {
    #[default]
    Gaussian,
}

#[derive(Default)]
pub struct DepthOfFieldNode {
    pub config: DepthOfField,
    pub mode: DepthOfFieldMode,

    pub data: Option<DepthOfFieldData>,
}

impl RenderNode for DepthOfFieldNode {
    fn require_shaders(&self) -> Option<&'static [(&'static [&'static str], &'static str)]> {
        Some(&[
            (&[], include_str!("../shader/fullscreen.wgsl")),
            (
                &[include_str!("../shader/fullscreen.wgsl")],
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

        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("dof_layout"),
            entries: &[
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
                        sample_type: TextureSampleType::Float { filterable: false },
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
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("deo_pipeline_layout"),
            bind_group_layouts: &[&layout],
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
        }
    }

    fn draw(
        &self,
        _scene: &mut GpuScene,
        RenderContext {
            device,
            queue,
            targets,
            ..
        }: RenderContext,
    ) {
        match self.data.as_ref().unwrap() {
            DepthOfFieldData::Gaussian(data) => {
                targets.post_process_chain.swap();
                let mut command_encoder = device.create_command_encoder(&Default::default());
                let mut bind_group_entries = [
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(targets.depth.as_ref().unwrap()),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(targets.post_process_chain.another_view()),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Sampler(&data.sampler),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: data.config.binding::<DepthOfField>().unwrap(),
                    },
                ];

                {
                    let bind_group_horizontal = device.create_bind_group(&BindGroupDescriptor {
                        label: Some("dof_bind_group_horizontal"),
                        layout: &data.layout,
                        entries: &bind_group_entries,
                    });

                    let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("dof_gaussian_horizontal"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: &targets.post_process_chain.current_view(),
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(Color::TRANSPARENT),
                                store: StoreOp::Store,
                            },
                        })],
                        ..Default::default()
                    });
                    pass.set_pipeline(&data.horizontal);
                    pass.set_bind_group(0, &bind_group_horizontal, &[]);
                    pass.draw(0..3, 0..1);
                }
                queue.submit([command_encoder.finish()]);

                targets.post_process_chain.swap();
                let mut command_encoder = device.create_command_encoder(&Default::default());

                {
                    bind_group_entries[1].resource =
                        BindingResource::TextureView(&targets.post_process_chain.another_view());
                    let bind_group_vertical = device.create_bind_group(&BindGroupDescriptor {
                        label: Some("dof_bind_group_vertical"),
                        layout: &data.layout,
                        entries: &bind_group_entries,
                    });

                    let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("dof_gaussian_vertical"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: &targets.post_process_chain.current_view(),
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(Color::TRANSPARENT),
                                store: StoreOp::Store,
                            },
                        })],
                        ..Default::default()
                    });

                    pass.set_pipeline(&data.vertical);
                    pass.set_bind_group(0, &bind_group_vertical, &[]);
                    pass.draw(0..3, 0..1);
                }
                queue.submit([command_encoder.finish()]);
            }
        }
    }
}
