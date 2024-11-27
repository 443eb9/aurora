use aurora_core::render::{
    flow::{RenderContext, RenderNode},
    resource::DynamicGpuBuffer,
    scene::GpuScene,
};
use encase::ShaderType;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BufferBindingType, BufferUsages,
    ColorTargetState, ColorWrites, FilterMode, FragmentState, PipelineLayoutDescriptor,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages, TextureSampleType,
    TextureViewDimension, VertexState,
};

use crate::node::MOTION_VECTOR_PREPASS_TEXTURE;

#[derive(ShaderType)]
pub struct MotionBlurConfig {
    pub strength: f32,
    pub samples: u32,
    pub frame: u32,
}

impl Default for MotionBlurConfig {
    fn default() -> Self {
        Self {
            strength: 2.0,
            samples: 20,
            frame: 0,
        }
    }
}

pub struct MotionBlurNodeData {
    pub pipeline: RenderPipeline,
    pub layout: BindGroupLayout,
    pub motion_vector_sampler: Sampler,
    pub color_sampler: Sampler,
    pub config: DynamicGpuBuffer,
}

pub struct MotionBlurNodeConfig {}

impl Default for MotionBlurNodeConfig {
    fn default() -> Self {
        Self {}
    }
}

#[derive(Default)]
pub struct MotionBlurNode {
    pub config: MotionBlurConfig,
    pub node_config: MotionBlurNodeConfig,

    pub data: Option<MotionBlurNodeData>,
}

impl RenderNode for MotionBlurNode {
    fn require_shaders(&self) -> Option<&'static [(&'static [&'static str], &'static str)]> {
        Some(&[
            (&[], include_str!("../shader/fullscreen.wgsl")),
            (
                &[
                    include_str!("../shader/fullscreen.wgsl"),
                    include_str!("../shader/math.wgsl"),
                ],
                include_str!("../shader/post_processing/motion_blur.wgsl"),
            ),
        ])
    }

    fn build(
        &mut self,
        _scene: &mut GpuScene,
        RenderContext {
            device,
            node,
            targets,
            ..
        }: RenderContext,
    ) {
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("motion_blur_layout"),
            entries: &[
                // Color
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
                // Motion Vector
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
                // Color Sampler
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                // Motion Vector Sampler
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                },
                // Config
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(MotionBlurConfig::min_size()),
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("motion_blur_pipeline_layout"),
            bind_group_layouts: &[&layout],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("motion_blur_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &node.shaders[0],
                entry_point: "vertex",
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &node.shaders[1],
                entry_point: "fragment",
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: targets.color_format,
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

        let motion_vector_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("motion_vector_sampler"),
            ..Default::default()
        });

        let color_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("color_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        let config = DynamicGpuBuffer::new(BufferUsages::UNIFORM);

        self.data = Some(MotionBlurNodeData {
            pipeline,
            layout,
            motion_vector_sampler,
            color_sampler,
            config,
        });
    }

    fn prepare(
        &mut self,
        GpuScene { frame_count, .. }: &mut GpuScene,
        RenderContext { device, queue, .. }: RenderContext,
    ) {
        let Some(MotionBlurNodeData { config, .. }) = &mut self.data else {
            return;
        };

        self.config.frame = *frame_count;
        config.clear();
        config.push(&self.config);
        config.write::<MotionBlurConfig>(device, queue);
    }

    fn draw(
        &self,
        GpuScene { assets, .. }: &mut GpuScene,
        RenderContext {
            device,
            queue,
            targets,
            ..
        }: RenderContext,
    ) {
        let Some(MotionBlurNodeData {
            pipeline,
            layout,
            motion_vector_sampler,
            color_sampler,
            config,
        }) = &self.data
        else {
            return;
        };

        let post_process = targets.swap_chain.start_post_process();
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("motion_blur_bind_group"),
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(post_process.src),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &assets.texture_views[&MOTION_VECTOR_PREPASS_TEXTURE.view],
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(color_sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(motion_vector_sampler),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: config.entire_binding().unwrap(),
                },
            ],
        });

        let mut command_encoder = device.create_command_encoder(&Default::default());

        {
            let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("motion_blur_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: post_process.dst,
                    resolve_target: None,
                    ops: Default::default(),
                })],
                ..Default::default()
            });

            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit([command_encoder.finish()]);
    }
}
