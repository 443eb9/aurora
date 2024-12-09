use std::path::PathBuf;

use aurora_core::render::{
    flow::{RenderContext, RenderNode},
    resource::{GpuCamera, Image},
    scene::GpuScene,
};
use encase::ShaderType;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BufferBindingType, ColorTargetState,
    ColorWrites, Features, FilterMode, FragmentState, PipelineLayoutDescriptor,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages, TextureSampleType, TextureView,
    TextureViewDescriptor, TextureViewDimension, VertexState,
};

pub struct SkyboxNodeData {
    pub pipeline: RenderPipeline,
    pub layout: BindGroupLayout,
    pub skybox: TextureView,
    pub sampler: Sampler,
}

pub struct SkyboxNodeConfig {
    pub skybox_path: PathBuf,
}

pub struct SkyboxNode {
    pub node_config: SkyboxNodeConfig,
    pub data: Option<SkyboxNodeData>,
}

impl RenderNode for SkyboxNode {
    fn require_renderer_features(&self, features: &mut Features) {
        *features |= Features::FLOAT32_FILTERABLE;
    }

    fn require_shaders(&self) -> Option<&'static [(&'static [&'static str], &'static str)]> {
        Some(&[
            (&[], include_str!("../shader/fullscreen.wgsl")),
            (
                &[
                    include_str!("../shader/fullscreen.wgsl"),
                    include_str!("../shader/common/common_type.wgsl"),
                ],
                include_str!("../shader/env_mapping/skybox.wgsl"),
            ),
        ])
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
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("skybox_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::Cube,
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
                        min_binding_size: Some(GpuCamera::min_size()),
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("skybox_pipeline_layout"),
            bind_group_layouts: &[&layout],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("skybox_pipeline"),
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

        let skybox = Image::from_path(&self.node_config.skybox_path, None, true)
            .unwrap()
            .to_cube_map(device, queue, &Default::default());

        let skybox = skybox.create_view(&TextureViewDescriptor {
            label: Some("skybox_view"),
            dimension: Some(TextureViewDimension::Cube),
            ..Default::default()
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("skybox_sampler"),
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        self.data = Some(SkyboxNodeData {
            pipeline,
            layout,
            skybox,
            sampler,
        });
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
        let Some(SkyboxNodeData {
            pipeline,
            layout,
            skybox,
            sampler,
        }) = &self.data
        else {
            return;
        };

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("skybox_bind_group"),
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&skybox),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: assets.camera_uniform.entire_binding().unwrap(),
                },
            ],
        });

        let mut command_encoder = device.create_command_encoder(&Default::default());

        {
            let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("skybox_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: targets.swap_chain.current_view(),
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
