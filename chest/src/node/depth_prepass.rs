use aurora_core::render::{
    flow::{RenderContext, RenderNode},
    scene::{GpuScene, TextureId, TextureViewId},
};
use uuid::Uuid;
use wgpu::{
    CompareFunction, DepthStencilState, Extent3d, FragmentState, LoadOp, Operations,
    PipelineLayoutDescriptor, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipelineDescriptor, StoreOp, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureViewDescriptor, VertexBufferLayout, VertexFormat, VertexState,
    VertexStepMode,
};

pub struct DepthPrepassTexture {
    pub texture: TextureId,
    pub view: TextureViewId,
}

pub const DEPTH_PREPASS_TEXTURE: DepthPrepassTexture = DepthPrepassTexture {
    texture: TextureId(Uuid::from_u128(849651230456123074856245)),
    view: TextureViewId(Uuid::from_u128(8978946514851414745)),
};

pub const DEPTH_PREPASS_FORMAT: TextureFormat = TextureFormat::Depth32Float;

#[derive(Default)]
pub struct DepthPrepassNode;

impl RenderNode for DepthPrepassNode {
    fn restrict_mesh_format(&self) -> Option<&'static [VertexFormat]> {
        Some(&[
            VertexFormat::Float32x3,
            VertexFormat::Float32x3,
            VertexFormat::Float32x2,
            VertexFormat::Float32x4,
        ])
    }

    fn require_shaders(&self) -> Option<&'static [(&'static [&'static str], &'static str)]> {
        Some(&[(
            &[
                include_str!("../shader/common/common_type.wgsl"),
                include_str!("../shader/common/common_binding.wgsl"),
            ],
            include_str!("../shader/prepass/depth_prepass.wgsl"),
        )])
    }

    fn build(
        &mut self,
        GpuScene { assets, .. }: &mut GpuScene,
        RenderContext {
            device,
            targets,
            node,
            ..
        }: RenderContext,
    ) {
        let depth_texture = device.create_texture(&TextureDescriptor {
            label: Some("depth_prepass_texture"),
            dimension: TextureDimension::D2,
            format: DEPTH_PREPASS_FORMAT,
            mip_level_count: 1,
            sample_count: 1,
            size: Extent3d {
                width: targets.size.x,
                height: targets.size.y,
                depth_or_array_layers: 1,
            },
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let depth_texture_view = depth_texture.create_view(&TextureViewDescriptor {
            label: Some("depth_prepass_texture_view"),
            ..Default::default()
        });

        assets
            .textures
            .insert(DEPTH_PREPASS_TEXTURE.texture, depth_texture);
        assets
            .texture_views
            .insert(DEPTH_PREPASS_TEXTURE.view, depth_texture_view);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("depth_prepass_pipeline_layout"),
            bind_group_layouts: &[assets.common_layout.as_ref().unwrap()],
            push_constant_ranges: &[],
        });

        for mesh in &node.meshes {
            if node.pipelines.contains_key(&mesh.mesh.mesh) {
                continue;
            }

            let instance = &assets.meshes[&mesh.mesh.mesh];
            let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("depth_prepass_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &node.shaders[0],
                    entry_point: "vertex",
                    compilation_options: Default::default(),
                    buffers: &[VertexBufferLayout {
                        array_stride: instance.vertex_stride(),
                        step_mode: VertexStepMode::Vertex,
                        attributes: &instance.vertex_attributes(),
                    }],
                },
                fragment: Some(FragmentState {
                    module: &node.shaders[0],
                    entry_point: "fragment",
                    compilation_options: Default::default(),
                    targets: &[None],
                }),
                primitive: Default::default(),
                depth_stencil: Some(DepthStencilState {
                    format: DEPTH_PREPASS_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview: Default::default(),
                cache: Default::default(),
            });
            node.pipelines.insert(mesh.mesh.mesh, pipeline);
        }
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
                label: Some("depth_prepass"),
                color_attachments: &[None],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &assets.texture_views[&DEPTH_PREPASS_TEXTURE.view],
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            pass.set_bind_group(0, assets.common_bind_group.as_ref().unwrap(), &[]);
            for mesh in &node.meshes {
                let (instance, pipeline) = (
                    &assets.gpu_meshes[&mesh.mesh.mesh],
                    &node.pipelines[&mesh.mesh.mesh],
                );

                pass.set_pipeline(pipeline);
                pass.set_vertex_buffer(0, instance.vertex_buffer.slice(..));
                if let Some(indices) = &instance.index_buffer {
                    pass.set_index_buffer(indices.buffer.slice(..), indices.format);
                    pass.draw_indexed(0..indices.count, 0, 0..1);
                } else {
                    pass.draw(0..instance.vertices_count, 0..1);
                }
            }
        }

        queue.submit([command_encoder.finish()]);
    }
}
