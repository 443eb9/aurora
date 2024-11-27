use aurora_core::render::{
    flow::{RenderContext, RenderNode},
    resource::{DynamicGpuBuffer, GpuCamera},
    scene::{GpuScene, TextureId, TextureViewId},
};
use encase::ShaderType;
use glam::Mat4;
use uuid::Uuid;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BufferBindingType, BufferUsages, ColorTargetState,
    ColorWrites, CompareFunction, DepthStencilState, Extent3d, FragmentState, LoadOp, Operations,
    PipelineLayoutDescriptor, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, RenderPipelineDescriptor, ShaderStages, StoreOp, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, VertexBufferLayout, VertexFormat, VertexState,
    VertexStepMode,
};

use crate::node::DEPTH_PREPASS_TEXTURE;

pub const MOTION_VECTOR_PREPASS_FORMAT: TextureFormat = TextureFormat::Rg16Float;

pub struct MotionVectorTexture {
    pub texture: TextureId,
    pub view: TextureViewId,
}

pub const MOTION_VECTOR_PREPASS_TEXTURE: MotionVectorTexture = MotionVectorTexture {
    texture: TextureId(Uuid::from_u128(235514559123004)),
    view: TextureViewId(Uuid::from_u128(711332160019988)),
};

#[derive(ShaderType)]
pub struct MotionVectorPrepassConfig {
    pub previous_view: Mat4,
}

pub struct MotionVectorPrepassNodeData {
    pub current_view: Mat4,
    pub previous_view: DynamicGpuBuffer,
    pub layout: BindGroupLayout,
}

#[derive(Default)]
pub struct MotionVectorPrepassNode {
    pub data: Option<MotionVectorPrepassNodeData>,
}

impl RenderNode for MotionVectorPrepassNode {
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
                include_str!("../shader/math.wgsl"),
            ],
            include_str!("../shader/prepass/motion_vector_prepass.wgsl"),
        )])
    }

    fn build(
        &mut self,
        GpuScene { assets, .. }: &mut GpuScene,
        RenderContext {
            device,
            node,
            targets,
            ..
        }: RenderContext,
    ) {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("motion_vector_prepass"),
            size: Extent3d {
                width: targets.size.x,
                height: targets.size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: MOTION_VECTOR_PREPASS_FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let view = texture.create_view(&Default::default());

        assets
            .textures
            .insert(MOTION_VECTOR_PREPASS_TEXTURE.texture, texture);
        assets
            .texture_views
            .insert(MOTION_VECTOR_PREPASS_TEXTURE.view, view);

        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("motion_vector_prepass_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(GpuCamera::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(MotionVectorPrepassConfig::min_size()),
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("motion_vector_prepass_pipeline_layout"),
            bind_group_layouts: &[&layout],
            ..Default::default()
        });

        self.data = Some(MotionVectorPrepassNodeData {
            current_view: Default::default(),
            previous_view: DynamicGpuBuffer::new(BufferUsages::UNIFORM),
            layout,
        });

        for mesh in &node.meshes {
            if node.pipelines.contains_key(&mesh.mesh.mesh) {
                continue;
            }

            let instance = &assets.meshes[&mesh.mesh.mesh];
            let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("motion_vector_prepass_pipeline"),
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
                    targets: &[Some(ColorTargetState {
                        format: MOTION_VECTOR_PREPASS_FORMAT,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: Some(DepthStencilState {
                    format: targets.depth_format.unwrap(),
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                multiview: None,
                cache: None,
            });

            node.pipelines.insert(mesh.mesh.mesh, pipeline);
        }
    }

    fn prepare(
        &mut self,
        GpuScene { original, .. }: &mut GpuScene,
        RenderContext { device, queue, .. }: RenderContext,
    ) {
        let Some(MotionVectorPrepassNodeData {
            current_view,
            previous_view,
            ..
        }) = &mut self.data
        else {
            return;
        };

        previous_view.clear();
        previous_view.push(&MotionVectorPrepassConfig {
            previous_view: *current_view,
        });
        previous_view.write::<MotionVectorPrepassConfig>(device, queue);
        *current_view = original.camera.transform.compute_matrix().inverse();
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
        let Some(MotionVectorPrepassNodeData {
            previous_view,
            layout,
            ..
        }) = &self.data
        else {
            return;
        };

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("motion_vector_prepass_bind_group"),
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: assets.camera_uniform.entire_binding().unwrap(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: previous_view.entire_binding().unwrap(),
                },
            ],
        });

        let mut command_encoder = device.create_command_encoder(&Default::default());

        {
            let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("motion_vector_prepass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &assets.texture_views[&MOTION_VECTOR_PREPASS_TEXTURE.view],
                    resolve_target: None,
                    ops: Default::default(),
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

            pass.set_bind_group(0, &bind_group, &[]);
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
