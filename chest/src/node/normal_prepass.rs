use aurora_core::render::{
    flow::{PipelineCreationContext, RenderContext, RenderNode},
    resource::GpuCamera,
    scene::{GpuScene, TextureId, TextureViewId},
};
use encase::ShaderType;
use uuid::Uuid;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BufferBindingType, Color, ColorTargetState, ColorWrites,
    CompareFunction, DepthBiasState, DepthStencilState, Extent3d, FragmentState, LoadOp,
    Operations, PipelineLayoutDescriptor, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipelineDescriptor, ShaderStages,
    StoreOp, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};

use crate::node::DEPTH_PREPASS_TEXTURE;

pub struct NormalPrepassTexture {
    pub texture: TextureId,
    pub view: TextureViewId,
}

pub const NORMAL_PREPASS_TEXTURE: NormalPrepassTexture = NormalPrepassTexture {
    texture: TextureId(Uuid::from_u128(87456135453120100496854)),
    view: TextureViewId(Uuid::from_u128(3540690463413654698451)),
};

pub const NORMAL_PREPASS_FORMAT: TextureFormat = TextureFormat::Rgb10a2Unorm;

#[derive(Default)]
pub struct NormalPrepassNode {
    layout: Option<BindGroupLayout>,
    bind_group: Option<BindGroup>,
}

impl RenderNode for NormalPrepassNode {
    fn require_shader(&self) -> Option<(&'static [&'static str], &'static str)> {
        Some((
            &[include_str!("../shader/common/common_type.wgsl")],
            include_str!("../shader/prepass/normal_prepass.wgsl"),
        ))
    }

    fn restrict_mesh_format(&self) -> Option<&'static [VertexFormat]> {
        Some(&[
            VertexFormat::Float32x3,
            VertexFormat::Float32x3,
            VertexFormat::Float32x2,
            VertexFormat::Float32x4,
        ])
    }

    fn create_pipelines(
        &mut self,
        GpuScene { assets, .. }: &mut GpuScene,
        PipelineCreationContext {
            device,
            shader,
            meshes,
            pipelines,
            targets,
            ..
        }: PipelineCreationContext,
    ) {
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("normal_prepass_layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(GpuCamera::min_size()),
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("normal_prepass_pipeline_layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });

        for mesh in meshes {
            if pipelines.contains_key(&mesh.mesh.mesh) {
                continue;
            }

            let instance = &assets.meshes[&mesh.mesh.mesh];
            pipelines.insert(
                mesh.mesh.mesh,
                device.create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some("normal_prepass_pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: VertexState {
                        module: shader,
                        entry_point: "vertex",
                        compilation_options: Default::default(),
                        buffers: &[VertexBufferLayout {
                            array_stride: instance.vertex_stride(),
                            step_mode: VertexStepMode::Vertex,
                            attributes: &instance.vertex_attributes(),
                        }],
                    },
                    fragment: Some(FragmentState {
                        module: shader,
                        entry_point: "fragment",
                        compilation_options: Default::default(),
                        targets: &[Some(ColorTargetState {
                            format: NORMAL_PREPASS_FORMAT,
                            blend: None,
                            write_mask: ColorWrites::all(),
                        })],
                    }),
                    primitive: Default::default(),
                    depth_stencil: Some(DepthStencilState {
                        format: targets.depth_format.unwrap(),
                        depth_write_enabled: true,
                        depth_compare: CompareFunction::LessEqual,
                        stencil: Default::default(),
                        bias: DepthBiasState::default(),
                    }),
                    multisample: Default::default(),
                    multiview: None,
                    cache: None,
                }),
            );
        }

        self.layout = Some(layout);
    }

    fn build(
        &mut self,
        GpuScene { assets, .. }: &mut GpuScene,
        RenderContext {
            device, targets, ..
        }: RenderContext,
    ) {
        let normal_texture = device.create_texture(&TextureDescriptor {
            label: Some("normal_prepass_texture"),
            size: Extent3d {
                width: targets.size.x,
                height: targets.size.y,
                depth_or_array_layers: 1,
            },
            dimension: TextureDimension::D2,
            format: NORMAL_PREPASS_FORMAT,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let normal_texture_view = normal_texture.create_view(&TextureViewDescriptor {
            label: Some("normal_prepass_texture_view"),
            ..Default::default()
        });

        assets
            .textures
            .insert(NORMAL_PREPASS_TEXTURE.texture, normal_texture);
        assets
            .texture_views
            .insert(NORMAL_PREPASS_TEXTURE.view, normal_texture_view);
    }

    fn prepare(
        &mut self,
        GpuScene { assets, .. }: &mut GpuScene,
        RenderContext { device, .. }: RenderContext,
    ) {
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("normal_prepass_bind_group"),
            layout: self.layout.as_ref().unwrap(),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: assets.camera_uniform.binding::<GpuCamera>().unwrap(),
            }],
        });

        self.bind_group = Some(bind_group);
    }

    fn draw(
        &self,
        GpuScene { assets, .. }: &mut GpuScene,
        RenderContext {
            device,
            queue,
            node,
            ..
        }: RenderContext,
    ) {
        let mut command_encoder = device.create_command_encoder(&Default::default());

        {
            let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("normal_prepass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &assets.texture_views[&NORMAL_PREPASS_TEXTURE.view],
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &assets.texture_views[&DEPTH_PREPASS_TEXTURE.view],
                    depth_ops: Some(Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            pass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);

            for mesh in &node.meshes {
                let instance = &assets.meshes[&mesh.mesh.mesh];
                let pipeline = &node.pipelines[&mesh.mesh.mesh];

                pass.set_pipeline(pipeline);
                pass.set_vertex_buffer(0, instance.create_vertex_buffer(device).unwrap().slice(..));
                if let Some(indices) = instance.create_index_buffer(device) {
                    pass.set_index_buffer(indices.buffer.slice(..), indices.format);
                    pass.draw_indexed(0..indices.count, 0, 0..1);
                } else {
                    pass.draw(0..instance.vertices_count() as u32, 0..1);
                }
            }
        }

        queue.submit([command_encoder.finish()]);
    }
}
