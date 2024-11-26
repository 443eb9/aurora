use std::collections::HashMap;

use aurora_core::render::{
    flow::{RenderContext, RenderNode},
    resource::DynamicGpuBuffer,
    scene::GpuScene,
};
use encase::ShaderType;
use naga_oil::compose::ShaderDefValue;
use wgpu::{
    AddressMode, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent, BlendFactor,
    BlendOperation, BlendState, BufferBindingType, BufferUsages, Color, ColorTargetState,
    ColorWrites, Extent3d, Features, FilterMode, FragmentState, LoadOp, Operations,
    PipelineLayoutDescriptor, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
    StoreOp, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, VertexState,
};

pub const BLOOM_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rg11b10Float;

#[derive(ShaderType)]
pub struct BloomConfig {
    pub precomputed_filter: [f32; 4],
}

pub struct BloomNodeData {
    pub first_downsampling_pipeline: RenderPipeline,
    pub downsampling_pipeline: RenderPipeline,
    pub upsampling_pipeline: RenderPipeline,
    pub final_upsampling_pipeline: RenderPipeline,

    pub config: DynamicGpuBuffer,
    pub pyramid_textures: Texture,
    pub texture_views: Vec<TextureView>,
    pub sampler: Sampler,
    pub layout: BindGroupLayout,
}

pub struct BloomNodeConfig {
    pub max_mip_dimension: u32,
    pub intensity: f32,
    pub scatter: f32,
    pub eliminate_firefly: bool,
    pub threshold: f32,
    pub soft_threshold: f32,
}

impl Default for BloomNodeConfig {
    fn default() -> Self {
        Self {
            max_mip_dimension: 512,
            intensity: 1.0,
            scatter: 0.8,
            eliminate_firefly: true,
            threshold: 0.8,
            soft_threshold: 0.9,
        }
    }
}

#[derive(Default)]
pub struct BloomNode {
    pub config: BloomNodeConfig,

    pub data: Option<BloomNodeData>,
}

impl BloomNode {
    pub fn calculate_blend_factor(&self, mip: usize) -> Color {
        let mip = mip as f32;
        let max_mip = self.data.as_ref().unwrap().texture_views.len() as f32 - 1.0;

        let BloomNodeConfig {
            intensity, scatter, ..
        } = self.config;

        let mut factor = (1.0 - (-(mip - max_mip / 2.0) / max_mip).abs()).powf(scatter);
        factor *= intensity;
        factor = factor.clamp(0.0, 1.0);

        let f = factor as f64;
        Color {
            r: f,
            g: f,
            b: f,
            a: f,
        }
    }
}

impl RenderNode for BloomNode {
    fn require_renderer_features(&self, features: &mut Features) {
        *features |= Features::RG11B10UFLOAT_RENDERABLE;
    }

    fn require_shaders(&self) -> Option<&'static [(&'static [&'static str], &'static str)]> {
        Some(&[
            (&[], include_str!("../shader/fullscreen.wgsl")),
            (
                &[
                    include_str!("../shader/math.wgsl"),
                    include_str!("../shader/fullscreen.wgsl"),
                ],
                include_str!("../shader/post_processing/bloom.wgsl"),
            ),
            (
                &[
                    include_str!("../shader/math.wgsl"),
                    include_str!("../shader/fullscreen.wgsl"),
                ],
                include_str!("../shader/post_processing/bloom.wgsl"),
            ),
        ])
    }

    fn require_shader_defs(&self, shader_defs: &mut HashMap<String, ShaderDefValue>) {
        if self.config.threshold > 0.0 {
            shader_defs.insert("SOFT_THRESHOLD".to_string(), Default::default());
        }
    }

    fn require_local_shader_defs(&self) -> Vec<Option<Vec<(String, ShaderDefValue)>>> {
        vec![
            None,
            self.config
                .eliminate_firefly
                .then(|| vec![("FIRST_DOWNSAMPLE".to_string(), Default::default())]),
        ]
    }

    fn build(
        &mut self,
        _scene: &mut GpuScene,
        RenderContext {
            device,
            queue,
            node,
            targets,
            ..
        }: RenderContext,
    ) {
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("bloom_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(BloomConfig::min_size()),
                    },
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("bloom_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("bloom_pipeline_layout"),
            bind_group_layouts: &[&layout],
            ..Default::default()
        });

        let first_downsampling_pipeline =
            device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("first_downsampling_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &node.shaders[0],
                    entry_point: "vertex",
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &node.shaders[1],
                    entry_point: "downsample",
                    compilation_options: Default::default(),
                    targets: &[Some(ColorTargetState {
                        format: BLOOM_TEXTURE_FORMAT,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: Default::default(),
                multisample: Default::default(),
                multiview: Default::default(),
                cache: Default::default(),
            });

        let downsampling_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("downsampling_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &node.shaders[0],
                entry_point: "vertex",
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &node.shaders[1],
                entry_point: "downsample",
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: BLOOM_TEXTURE_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: Default::default(),
            multisample: Default::default(),
            multiview: Default::default(),
            cache: Default::default(),
        });

        let upsampling_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("upsampling_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &node.shaders[0],
                entry_point: "vertex",
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &node.shaders[2],
                entry_point: "upsample",
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: BLOOM_TEXTURE_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: Default::default(),
            multisample: Default::default(),
            multiview: Default::default(),
            cache: Default::default(),
        });

        let final_upsampling_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("final_upsampling_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &node.shaders[0],
                entry_point: "vertex",
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &node.shaders[2],
                entry_point: "upsample",
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: targets.color_format,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::Constant,
                            dst_factor: BlendFactor::OneMinusConstant,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::Zero,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    // blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: Default::default(),
            multisample: Default::default(),
            multiview: Default::default(),
            cache: Default::default(),
        });

        let mip_count = self.config.max_mip_dimension.ilog2().max(2) - 1;
        let scale =
            self.config.max_mip_dimension as f32 / targets.size.x.min(targets.size.y) as f32;

        let pyramid_textures = device.create_texture(&TextureDescriptor {
            label: Some("bloom_pyramid_textures"),
            size: Extent3d {
                width: (targets.size.x as f32 * scale).round() as u32,
                height: (targets.size.y as f32 * scale).round() as u32,
                depth_or_array_layers: 1,
            },
            dimension: TextureDimension::D2,
            format: BLOOM_TEXTURE_FORMAT,
            mip_level_count: mip_count,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let texture_views = (0..mip_count)
            .map(|mip| {
                pyramid_textures.create_view(&TextureViewDescriptor {
                    label: Some(&format!("bloom_pyramid_texture_mip{}", mip)),
                    base_mip_level: mip,
                    mip_level_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();

        let mut config = DynamicGpuBuffer::new(BufferUsages::UNIFORM);
        let knee = self.config.soft_threshold * self.config.threshold;
        config.push(&BloomConfig {
            precomputed_filter: [
                self.config.threshold,
                self.config.threshold - knee,
                2.0 * knee,
                0.25 / (knee + 0.00001),
            ],
        });
        config.write::<BloomConfig>(device, queue);

        self.data = Some(BloomNodeData {
            first_downsampling_pipeline,
            downsampling_pipeline,
            upsampling_pipeline,
            final_upsampling_pipeline,
            config,
            pyramid_textures,
            texture_views,
            sampler,
            layout,
        });
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
        let Some(BloomNodeData {
            first_downsampling_pipeline,
            downsampling_pipeline,
            upsampling_pipeline,
            final_upsampling_pipeline,
            config,
            texture_views,
            sampler,
            layout,
            ..
        }) = &self.data
        else {
            return;
        };

        let first_downsample_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("bloom_first_downsample_bind_group"),
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(targets.swap_chain.current_view()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: config.entire_binding().unwrap(),
                },
            ],
        });

        let mut command_encoder = device.create_command_encoder(&Default::default());

        {
            let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("bloom_first_downsample_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &texture_views[0],
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                ..Default::default()
            });

            pass.set_pipeline(first_downsampling_pipeline);
            pass.set_bind_group(0, &first_downsample_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit([command_encoder.finish()]);

        for mip in 1..texture_views.len() {
            let downsample_bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some("bloom_downsample_bind_group"),
                layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&texture_views[mip - 1]),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: config.entire_binding().unwrap(),
                    },
                ],
            });

            let mut command_encoder = device.create_command_encoder(&Default::default());

            {
                let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("bloom_downsample_pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &texture_views[mip],
                        resolve_target: None,
                        ops: Operations::default(),
                    })],
                    ..Default::default()
                });

                pass.set_pipeline(downsampling_pipeline);
                pass.set_bind_group(0, &downsample_bind_group, &[]);
                pass.draw(0..3, 0..1);
            }

            queue.submit([command_encoder.finish()]);
        }

        for mip in (1..texture_views.len()).rev() {
            let upsample_bind_group = device.create_bind_group(&BindGroupDescriptor {
                label: Some("bloom_upsample_pass"),
                layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&texture_views[mip]),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: config.entire_binding().unwrap(),
                    },
                ],
            });

            let mut command_encoder = device.create_command_encoder(&Default::default());

            {
                let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("bloom_upsample_pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &texture_views[mip - 1],
                        resolve_target: None,
                        ops: Operations::default(),
                    })],
                    ..Default::default()
                });

                pass.set_pipeline(upsampling_pipeline);
                pass.set_blend_constant(self.calculate_blend_factor(mip));
                pass.set_bind_group(0, &upsample_bind_group, &[]);
                pass.draw(0..3, 0..1);
            }

            queue.submit([command_encoder.finish()]);
        }

        let final_upsample_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("bloom_upsample_pass"),
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture_views[0]),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: config.entire_binding().unwrap(),
                },
            ],
        });

        let mut command_encoder = device.create_command_encoder(&Default::default());

        {
            let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("bloom_final_upsample_pass"),
                color_attachments: &[Some({
                    RenderPassColorAttachment {
                        view: targets.swap_chain.current_view(),
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }
                })],
                ..Default::default()
            });

            pass.set_pipeline(final_upsampling_pipeline);
            pass.set_blend_constant(self.calculate_blend_factor(0));
            pass.set_bind_group(0, &final_upsample_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit([command_encoder.finish()]);
    }
}
