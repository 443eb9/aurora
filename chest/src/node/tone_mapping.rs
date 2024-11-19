use std::collections::HashMap;

use aurora_core::render::{
    flow::{RenderContext, RenderNode},
    scene::GpuScene,
    ShaderDefEnum,
};
use aurora_derive::ShaderDefEnum;
use naga_oil::compose::ShaderDefValue;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Color, ColorTargetState, ColorWrites,
    FilterMode, FragmentState, LoadOp, Operations, PipelineLayoutDescriptor,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages, StoreOp, Texture,
    TextureSampleType, TextureViewDimension, VertexState,
};

use crate::texture::load_dds_texture;

#[derive(ShaderDefEnum, Default)]
pub enum TonemappingMethod {
    Reinhard,
    #[default]
    TonyMcMapface,
}

pub struct TonemappingNodeData {
    pub pipeline: RenderPipeline,
    pub layout: BindGroupLayout,
    pub lut_sampler: Sampler,
    pub color_sampler: Sampler,
    pub lut: Texture,
}

#[derive(Default)]
pub struct TonemappingNode {
    pub method: Option<TonemappingMethod>,

    pub data: Option<TonemappingNodeData>,
}

impl RenderNode for TonemappingNode {
    fn require_shader_defs(&self, shader_defs: &mut HashMap<String, ShaderDefValue>) {
        if let Some(method) = &self.method {
            shader_defs.extend([method.to_def()]);
        }
    }

    fn require_shaders(&self) -> Option<&'static [(&'static [&'static str], &'static str)]> {
        Some(&[
            (&[], include_str!("../shader/fullscreen.wgsl")),
            (
                &[include_str!("../shader/fullscreen.wgsl")],
                include_str!("../shader/post_processing/tonemapping.wgsl"),
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
            ..
        }: RenderContext,
    ) {
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("tonemapping_layout"),
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
                // LUT
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D3,
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
                // LUT Sampler
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            bind_group_layouts: &[&layout],
            ..Default::default()
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("tonemapping_pipeline"),
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
                    format: targets.surface_format,
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

        let lut_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("lut_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        let color_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("color_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        let lut = load_dds_texture(device, queue, "chest/assets/luts/tony_mc_mapface.dds");

        self.data = Some(TonemappingNodeData {
            pipeline,
            layout,
            lut_sampler,
            color_sampler,
            lut,
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
        let data = self.data.as_ref().unwrap();
        let post_process = targets.swap_chain.start_post_process();
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("tonemapping_bind_group"),
            layout: &data.layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(post_process.src),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &data.lut.create_view(&Default::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&data.color_sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&data.lut_sampler),
                },
            ],
        });

        let data = self.data.as_ref().unwrap();
        let mut command_encoder = device.create_command_encoder(&Default::default());

        {
            let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("tonemapping_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &targets.surface,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            pass.set_pipeline(&data.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        queue.submit([command_encoder.finish()]);
    }
}
