use std::{any::TypeId, borrow::Cow, collections::HashMap};

use aurora_core::{
    render::{
        flow::RenderNode,
        resource::{RenderTarget, Vertex, CAMERA_UUID, LIGHTS_BIND_GROUP_UUID},
        scene::GpuScene,
    },
    scene::entity::StaticMesh,
    util::TypeIdAsUuid,
    WgpuRenderer,
};
use naga_oil::compose::{Composer, NagaModuleDescriptor, ShaderDefValue, ShaderType};
use wgpu::{
    vertex_attr_array, BufferAddress, Color, ColorTargetState, ColorWrites,
    CommandEncoderDescriptor, CompareFunction, DepthBiasState, DepthStencilState, FragmentState,
    LoadOp, MultisampleState, Operations, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor,
    ShaderSource, StencilState, StoreOp, TextureFormat, VertexBufferLayout, VertexState,
    VertexStepMode,
};

use crate::material::PbrMaterial;

#[derive(Default)]
pub struct BasicTriangleNode {
    pipeline: Option<RenderPipeline>,
}

impl RenderNode for BasicTriangleNode {
    fn build(
        &mut self,
        renderer: &WgpuRenderer,
        _scene: &GpuScene,
        _shader_defs: Option<HashMap<String, ShaderDefValue>>,
    ) {
        let shader_module = renderer
            .device
            .create_shader_module(ShaderModuleDescriptor {
                label: None,
                source: ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                    "shader/basic_triangle.wgsl"
                ))),
            });

        self.pipeline = Some(
            renderer
                .device
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label: None,
                    layout: None,
                    vertex: VertexState {
                        module: &shader_module,
                        entry_point: "vertex",
                        compilation_options: PipelineCompilationOptions::default(),
                        buffers: &[],
                    },
                    fragment: Some(FragmentState {
                        module: &shader_module,
                        entry_point: "fragment",
                        compilation_options: PipelineCompilationOptions::default(),
                        targets: &[Some(ColorTargetState {
                            format: TextureFormat::Bgra8UnormSrgb,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    primitive: PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    multiview: None,
                }),
        );
    }

    fn prepare(&mut self, _renderer: &WgpuRenderer, _scene: &mut GpuScene, _queue: &[StaticMesh]) {}

    fn draw(
        &self,
        renderer: &WgpuRenderer,
        _scene: &mut GpuScene,
        _queue: &[StaticMesh],
        target: &RenderTarget,
    ) {
        let mut encoder = renderer
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &target.color,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::GREEN),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            pass.set_pipeline(self.pipeline.as_ref().unwrap());
            pass.draw(0..3, 0..1);
        }

        renderer.queue.submit(Some(encoder.finish()));
    }
}

#[derive(Default)]
pub struct PbrNode {
    pipeline: Option<RenderPipeline>,
}

impl RenderNode for PbrNode {
    fn build(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &GpuScene,
        shader_defs: Option<HashMap<String, ShaderDefValue>>,
    ) {
        let assets = &scene.assets;

        let (Some(l_camera), Some(l_lights), Some(l_material)) = (
            assets.layouts.get(&CAMERA_UUID),
            assets.layouts.get(&LIGHTS_BIND_GROUP_UUID),
            assets.layouts.get(&TypeId::of::<PbrMaterial>().to_uuid()),
        ) else {
            return;
        };

        let mut composer = Composer::default();
        let shader = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("shader/pbr/pbr.wgsl"),
                file_path: "",
                shader_type: ShaderType::Wgsl,
                shader_defs: shader_defs.unwrap_or_default(),
                additional_imports: &[],
            })
            .unwrap();

        let module = renderer
            .device
            .create_shader_module(ShaderModuleDescriptor {
                label: None,
                source: ShaderSource::Naga(Cow::Owned(shader)),
            });

        let layout = renderer
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&l_camera, &l_lights, &l_material],
                push_constant_ranges: &[],
            });

        self.pipeline = Some(
            renderer
                .device
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label: None,
                    layout: Some(&layout),
                    vertex: VertexState {
                        module: &module,
                        entry_point: "vertex",
                        compilation_options: PipelineCompilationOptions::default(),
                        buffers: &[VertexBufferLayout {
                            array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
                            step_mode: VertexStepMode::Vertex,
                            attributes: &vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                        }],
                    },
                    multisample: MultisampleState::default(),
                    fragment: Some(FragmentState {
                        module: &module,
                        entry_point: "fragment",
                        compilation_options: PipelineCompilationOptions::default(),
                        targets: &[Some(ColorTargetState {
                            format: TextureFormat::Bgra8UnormSrgb,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    depth_stencil: Some(DepthStencilState {
                        format: TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: CompareFunction::LessEqual,
                        stencil: StencilState::default(),
                        bias: DepthBiasState::default(),
                    }),
                    primitive: PrimitiveState::default(),
                    multiview: None,
                }),
        )
    }

    fn prepare(&mut self, renderer: &WgpuRenderer, scene: &mut GpuScene, queue: &[StaticMesh]) {}

    fn draw(
        &self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        queue: &[StaticMesh],
        target: &RenderTarget,
    ) {
        let assets = &mut scene.assets;

        let mut encoder = renderer
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        let (Some(b_camera), Some(b_lights)) = (
            &assets.bind_groups.get(&CAMERA_UUID),
            &assets.bind_groups.get(&LIGHTS_BIND_GROUP_UUID),
        ) else {
            return;
        };

        let Some(pipeline) = &self.pipeline else {
            return;
        };

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &target.color,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: target.depth.as_ref().unwrap(),
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, b_camera, &[]);
            pass.set_bind_group(1, b_lights, &[]);

            for mesh in queue {
                let (Some(b_material), Some(mesh)) = (
                    assets.bind_groups.get(&mesh.material),
                    assets.buffers.get(&mesh.mesh),
                ) else {
                    continue;
                };

                pass.set_bind_group(3, b_material, &[]);
                pass.set_vertex_buffer(0, mesh.buffer().unwrap().slice(..));
                pass.draw(
                    0..mesh.len(std::mem::size_of::<Vertex>()).unwrap() as u32,
                    0..1,
                );
                println!("Draw mesh");
            }
        }

        renderer.queue.submit(Some(encoder.finish()));
    }
}
