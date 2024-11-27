use std::collections::HashMap;

use aurora_core::render::{
    flow::{RenderContext, RenderNode},
    resource::{DynamicGpuBuffer, Image, ImageTextureDescriptor},
    scene::GpuScene,
};
use encase::ShaderType;
use naga_oil::compose::ShaderDefValue;
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent,
    BlendFactor, BlendOperation, BlendState, BufferBindingType, BufferUsages, ColorTargetState,
    ColorWrites, Extent3d, Features, FilterMode, FragmentState, LoadOp, Operations,
    PipelineLayoutDescriptor, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
    StoreOp, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureView, TextureViewDescriptor, TextureViewDimension, VertexState,
};

pub const LENS_FLARE_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

#[derive(ShaderType)]
pub struct LensFlareConfig {
    pub spot_count: u32,
    pub center_falloff: f32,
    pub lower_threshold: f32,
    pub upper_threshold: f32,
    pub ca_strength: f32,
    pub halo_radius: f32,
}

impl Default for LensFlareConfig {
    fn default() -> Self {
        Self {
            spot_count: 2,
            center_falloff: 3.0,
            lower_threshold: 0.5,
            upper_threshold: 1.5,
            ca_strength: 20.0,
            halo_radius: 0.4,
        }
    }
}

pub struct LensFlareNodeConfig {
    pub downsample_scale: f32,
    pub chromatic_aberration: bool,
    pub halo: bool,
    pub startburst: bool,
}

impl Default for LensFlareNodeConfig {
    fn default() -> Self {
        Self {
            downsample_scale: 1.0 / 8.0,
            chromatic_aberration: true,
            halo: true,
            startburst: true,
        }
    }
}

pub struct LensFlareNodeData {
    pub downsample_pipeline: RenderPipeline,
    pub effect_pipeline: RenderPipeline,
    pub upsample_pipeline: RenderPipeline,

    pub blit_layout: BindGroupLayout,
    pub effect_bind_group: BindGroup,
    pub downsample_output: TextureView,
    pub effect_output: TextureView,
    pub sampler: Sampler,
}

#[derive(Default)]
pub struct LensFlareNode {
    pub config: LensFlareConfig,
    pub node_config: LensFlareNodeConfig,

    pub data: Option<LensFlareNodeData>,
}

impl RenderNode for LensFlareNode {
    fn require_shader_defs(&self, shader_defs: &mut HashMap<String, ShaderDefValue>) {
        if self.node_config.chromatic_aberration {
            shader_defs.insert("CHROMATIC_ABERRATION".to_string(), Default::default());
        }

        if self.node_config.halo {
            shader_defs.insert("HALO".to_string(), Default::default());
        }

        if self.node_config.startburst {
            shader_defs.insert("STAR_BURST".to_string(), Default::default());
        }
    }

    fn require_shaders(&self) -> Option<&'static [(&'static [&'static str], &'static str)]> {
        Some(&[
            (&[], include_str!("../shader/fullscreen.wgsl")),
            (
                &[
                    include_str!("../shader/fullscreen.wgsl"),
                    include_str!("../shader/math.wgsl"),
                    include_str!("../shader/hash.wgsl"),
                ],
                include_str!("../shader/post_processing/lens_flare.wgsl"),
            ),
        ])
    }

    fn require_renderer_features(&self, features: &mut Features) {
        *features |= Features::RG11B10UFLOAT_RENDERABLE;
    }

    fn build(
        &mut self,
        _scene: &mut GpuScene,
        RenderContext {
            device,
            queue,
            node,
            targets,
        }: RenderContext,
    ) {
        let blit_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("lens_flare_blit_layout"),
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
            ],
        });

        let blit_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("lens_flare_blit_pipeline_layout"),
            bind_group_layouts: &[&blit_layout],
            ..Default::default()
        });

        let downsample_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("lens_flare_downsample_pipeline"),
            layout: Some(&blit_pipeline_layout),
            vertex: VertexState {
                module: &node.shaders[0],
                entry_point: "vertex",
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &node.shaders[1],
                entry_point: "blit",
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: LENS_FLARE_TEXTURE_FORMAT,
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

        let upsample_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("lens_flare_upsample_pipeline"),
            layout: Some(&blit_pipeline_layout),
            vertex: VertexState {
                module: &node.shaders[0],
                entry_point: "vertex",
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &node.shaders[1],
                entry_point: "blit",
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: targets.color_format,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::Zero,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: Default::default(),
            multisample: Default::default(),
            multiview: Default::default(),
            cache: Default::default(),
        });

        let effect_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("lens_flare_layout"),
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
                        min_binding_size: Some(LensFlareConfig::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D1,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let effect_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("lens_flare_effect_pipeline_layout"),
            bind_group_layouts: &[&effect_layout],
            ..Default::default()
        });

        let effect_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("lens_flare_effect_pipeline"),
            layout: Some(&effect_pipeline_layout),
            vertex: VertexState {
                module: &node.shaders[0],
                entry_point: "vertex",
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &node.shaders[1],
                entry_point: "lens_flare",
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: LENS_FLARE_TEXTURE_FORMAT,
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

        let desc = TextureDescriptor {
            label: Some("lens_flare_texture"),
            dimension: TextureDimension::D2,
            format: LENS_FLARE_TEXTURE_FORMAT,
            size: Extent3d {
                width: (targets.size.x as f32 * self.node_config.downsample_scale).round() as u32,
                height: (targets.size.y as f32 * self.node_config.downsample_scale).round() as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };
        let downsample_output = device.create_texture(&desc);
        let effect_output = device.create_texture(&desc);

        let downsample_output = downsample_output.create_view(&TextureViewDescriptor {
            label: Some("lens_flare_downsample_output_view"),
            ..Default::default()
        });

        let effect_output = effect_output.create_view(&TextureViewDescriptor {
            label: Some("lens_flare_effect_downsample_view"),
            ..Default::default()
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("lens_flare_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            ..Default::default()
        });

        let mut config = DynamicGpuBuffer::new(BufferUsages::UNIFORM);
        config.push(&self.config);
        config.write::<LensFlareConfig>(device, queue);

        let starburst_image = Image::from_path("chest/assets/starburst.png").unwrap();
        let starburst_texture = starburst_image.to_texture(
            device,
            queue,
            &ImageTextureDescriptor {
                dimension: Some(TextureDimension::D1),
                ..Default::default()
            },
        );
        let starburst_view = starburst_texture.create_view(&Default::default());

        let effect_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("lens_flare_effect_bind_group"),
            layout: &effect_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&downsample_output),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: config.entire_binding().unwrap(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&starburst_view),
                },
            ],
        });

        self.data = Some(LensFlareNodeData {
            downsample_pipeline,
            upsample_pipeline,
            effect_pipeline,
            blit_layout,
            effect_bind_group,
            sampler,
            downsample_output,
            effect_output,
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
        let LensFlareNodeData {
            downsample_pipeline,
            effect_pipeline,
            upsample_pipeline,
            blit_layout,
            downsample_output,
            effect_output,
            effect_bind_group,
            sampler,
        } = self.data.as_ref().unwrap();

        let downsample_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("lens_flare_downsample_bind_group"),
            layout: blit_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(targets.swap_chain.current_view()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
        });

        let mut command_encoder = device.create_command_encoder(&Default::default());

        {
            let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("lens_flare_downsample_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: downsample_output,
                    resolve_target: None,
                    ops: Default::default(),
                })],
                ..Default::default()
            });

            pass.set_pipeline(downsample_pipeline);
            pass.set_bind_group(0, &downsample_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit([command_encoder.finish()]);

        let mut command_encoder = device.create_command_encoder(&Default::default());

        {
            let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("lens_flare_effect_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: effect_output,
                    resolve_target: None,
                    ops: Default::default(),
                })],
                ..Default::default()
            });

            pass.set_pipeline(effect_pipeline);
            pass.set_bind_group(0, &effect_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit([command_encoder.finish()]);

        let upsample_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("lens_flare_upsample_bind_group"),
            layout: blit_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(effect_output),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
        });

        let mut command_encoder = device.create_command_encoder(&Default::default());

        {
            let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("lens_flare_upsample_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: targets.swap_chain.current_view(),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            pass.set_pipeline(upsample_pipeline);
            pass.set_bind_group(0, &upsample_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit([command_encoder.finish()]);
    }
}
