use std::{
    any::TypeId,
    borrow::Cow,
    collections::{hash_map::Entry, HashMap},
};

use aurora_core::{
    render::{
        flow::RenderNode,
        resource::{
            DynamicGpuBuffer, RenderMesh, RenderTarget, Vertex, CAMERA_UUID,
            LIGHTS_BIND_GROUP_UUID, POST_PROCESS_DEPTH_LAYOUT_UUID,
        },
        scene::GpuScene,
    },
    util::TypeIdAsUuid,
    WgpuRenderer,
};
use naga_oil::compose::{
    ComposableModuleDescriptor, Composer, NagaModuleDescriptor, ShaderDefValue, ShaderLanguage,
    ShaderType,
};
use uuid::Uuid;
use wgpu::{
<<<<<<< Updated upstream
    vertex_attr_array, BindGroupDescriptor, BindGroupEntry, BindingResource, BufferAddress,
    BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor, CompareFunction,
    DepthBiasState, DepthStencilState, Face, FilterMode, FragmentState, LoadOp, MultisampleState,
    Operations, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerDescriptor, ShaderModuleDescriptor,
    ShaderSource, StencilState, StoreOp, TextureFormat, VertexBufferLayout, VertexState,
    VertexStepMode,
=======
    vertex_attr_array, BindGroupDescriptor, BindGroupEntry, BindingResource, BufferAddress,
    BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor, CompareFunction,
    DepthBiasState, DepthStencilState, Face, FilterMode, FragmentState, LoadOp, MultisampleState,
    Operations, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerDescriptor, ShaderModuleDescriptor,
    ShaderSource, StencilState, StoreOp, VertexBufferLayout, VertexState, VertexStepMode,
>>>>>>> Stashed changes
};

use crate::material::PbrMaterial;
const CLEAR_COLOR: Color = Color {
    r: 43. / 255.,
    g: 44. / 255.,
    b: 47. / 255.,
    a: 1.,
};

#[derive(Default)]
pub struct BasicTriangleNode {
    pipeline: Option<RenderPipeline>,
}

impl RenderNode for BasicTriangleNode {
    fn build(
        &mut self,
        renderer: &WgpuRenderer,
        _scene: &mut GpuScene,
        _shader_defs: Option<HashMap<String, ShaderDefValue>>,
        target: &RenderTarget,
    ) {
        let shader_module = renderer
            .device
            .create_shader_module(ShaderModuleDescriptor {
                label: Some("basic_triangle_shader"),
                source: ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                    "shader/basic_triangle.wgsl"
                ))),
            });

        self.pipeline = Some(
            renderer
                .device
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some("basic_triagle_pipeline"),
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
                            format: target.color_format,
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

    fn prepare(
        &mut self,
        _renderer: &WgpuRenderer,
        _scene: &mut GpuScene,
        _queue: &mut [RenderMesh],
        _target: &RenderTarget,
    ) {
    }

    fn draw(
        &self,
        renderer: &WgpuRenderer,
        _scene: &GpuScene,
        _queue: &[RenderMesh],
        target: &RenderTarget,
    ) {
        let mut encoder = renderer
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("basic_triangle_pass"),
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
    mat_uuid: Uuid,
    pipeline: Option<RenderPipeline>,
}

impl RenderNode for PbrNode {
    fn build(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        shader_defs: Option<HashMap<String, ShaderDefValue>>,
        target: &RenderTarget,
    ) {
        let assets = &scene.assets;

        self.mat_uuid = TypeId::of::<PbrMaterial>().to_uuid();

        let (l_camera, l_lights, l_material) = (
            assets.layouts.get(&CAMERA_UUID).unwrap(),
            assets.layouts.get(&LIGHTS_BIND_GROUP_UUID).unwrap(),
            assets.layouts.get(&self.mat_uuid).unwrap(),
        );

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
                label: Some("pbr_shader"),
                source: ShaderSource::Naga(Cow::Owned(shader)),
            });

        let layout = renderer
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("pbr_pipeline_layout"),
                bind_group_layouts: &[&l_camera, &l_lights, &l_material],
                push_constant_ranges: &[],
            });

        self.pipeline = Some(
            renderer
                .device
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some("pbr_pipeline"),
                    layout: Some(&layout),
                    vertex: VertexState {
                        module: &module,
                        entry_point: "vertex",
                        compilation_options: PipelineCompilationOptions::default(),
                        buffers: &[VertexBufferLayout {
                            array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
                            step_mode: VertexStepMode::Vertex,
                            attributes: &vertex_attr_array![
                                // Position
                                0 => Float32x3,
                                // Normal
                                1 => Float32x3,
                                // UV
                                2 => Float32x3
                            ],
                        }],
                    },
                    multisample: MultisampleState::default(),
                    fragment: Some(FragmentState {
                        module: &module,
                        entry_point: "fragment",
                        compilation_options: PipelineCompilationOptions::default(),
                        targets: &[Some(ColorTargetState {
                            format: target.color_format,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    depth_stencil: Some(DepthStencilState {
                        format: target.depth_format.unwrap(),
                        depth_write_enabled: true,
                        depth_compare: CompareFunction::LessEqual,
                        stencil: StencilState::default(),
                        bias: DepthBiasState::default(),
                    }),
                    primitive: PrimitiveState {
                        cull_mode: Some(Face::Back),
                        ..Default::default()
                    },
                    multiview: None,
                }),
        )
    }

    fn prepare(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        queue: &mut [RenderMesh],
        _target: &RenderTarget,
    ) {
        match scene.assets.buffers.entry(self.mat_uuid) {
            Entry::Occupied(mut e) => e.get_mut().clear(),
            Entry::Vacant(e) => {
                e.insert(DynamicGpuBuffer::new(BufferUsages::UNIFORM));
            }
        }

        queue
            .iter_mut()
            .filter_map(|sm| scene.materials.get(&sm.mesh.material).map(|m| (m, sm)))
            .for_each(|(material, mesh)| {
                mesh.offset = Some(material.prepare(renderer, &mut scene.assets));
            });

        scene
            .assets
            .buffers
            .get_mut(&self.mat_uuid)
            .unwrap()
            .write(&renderer.device, &renderer.queue);

        queue
            .iter_mut()
            .filter_map(|sm| scene.materials.get(&sm.mesh.material).map(|m| (m, sm)))
            .for_each(|(material, mesh)| {
                material.create_bind_group(renderer, &mut scene.assets, mesh.mesh.material);
            });
    }

    fn draw(
        &self,
        renderer: &WgpuRenderer,
        scene: &GpuScene,
        queue: &[RenderMesh],
        target: &RenderTarget,
    ) {
        let assets = &scene.assets;

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
                label: Some("pbr_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &target.color,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(CLEAR_COLOR),
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
                let (Some(b_material), Some(vertices)) = (
                    assets.bind_groups.get(&mesh.mesh.material),
                    assets.buffers.get(&mesh.mesh.mesh),
                ) else {
                    continue;
                };

                if let Some(offset) = mesh.offset {
                    pass.set_bind_group(2, b_material, &[offset]);
                } else {
                    pass.set_bind_group(2, b_material, &[]);
                }
                pass.set_vertex_buffer(0, vertices.buffer().unwrap().slice(..));
                pass.draw(0..vertices.len::<Vertex>().unwrap() as u32, 0..1);
            }
        }

        renderer.queue.submit(Some(encoder.finish()));
    }
}
<<<<<<< Updated upstream
=======

#[derive(Default)]
pub struct DepthViewNode {
    pipeline: Option<RenderPipeline>,
    sampler: Option<Sampler>,
}

impl RenderNode for DepthViewNode {
    fn build(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        _shader_defs: Option<HashMap<String, ShaderDefValue>>,
        target: &RenderTarget,
    ) {
        let Some(l_post_process) = scene.assets.layouts.get(&POST_PROCESS_DEPTH_LAYOUT_UUID) else {
            return;
        };

        let layout = renderer
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("depth_view_pipeline_layout"),
                bind_group_layouts: &[l_post_process],
                push_constant_ranges: &[],
            });

        let mut composer = Composer::default();
        composer
            .add_composable_module(ComposableModuleDescriptor {
                source: include_str!("shader/fullscreen.wgsl"),
                file_path: "",
                language: ShaderLanguage::Wgsl,
                shader_defs: HashMap::default(),
                additional_imports: &[],
                as_name: None,
            })
            .unwrap();

        let vert_shader = Composer::default()
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("shader/fullscreen.wgsl"),
                file_path: "",
                shader_type: ShaderType::Wgsl,
                shader_defs: HashMap::default(),
                additional_imports: &[],
            })
            .unwrap();
        let vert_module = renderer
            .device
            .create_shader_module(ShaderModuleDescriptor {
                label: Some("fullscreen_vertex_shader"),
                source: ShaderSource::Naga(Cow::Owned(vert_shader)),
            });

        let frag_shader = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("shader/depth_view.wgsl"),
                file_path: "",
                shader_type: ShaderType::Wgsl,
                shader_defs: HashMap::default(),
                additional_imports: &[],
            })
            .unwrap();
        let frag_module = renderer
            .device
            .create_shader_module(ShaderModuleDescriptor {
                label: Some("depth_view_shader"),
                source: ShaderSource::Naga(Cow::Owned(frag_shader)),
            });

        self.pipeline = Some(
            renderer
                .device
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some("depth_view_pipeline"),
                    layout: Some(&layout),
                    vertex: VertexState {
                        module: &vert_module,
                        entry_point: "vertex",
                        compilation_options: PipelineCompilationOptions::default(),
                        buffers: &[],
                    },
                    fragment: Some(FragmentState {
                        module: &frag_module,
                        entry_point: "fragment",
                        compilation_options: PipelineCompilationOptions::default(),
                        targets: &[Some(ColorTargetState {
                            format: target.color_format,
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

        self.sampler = Some(renderer.device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        }));
    }

    fn prepare(
        &mut self,
        _renderer: &WgpuRenderer,
        _scene: &mut GpuScene,
        _queue: &mut [RenderMesh],
        _target: &RenderTarget,
    ) {
    }

    fn draw(
        &self,
        renderer: &WgpuRenderer,
        scene: &GpuScene,
        _queue: &[RenderMesh],
        target: &RenderTarget,
    ) {
        let mut encoder = renderer
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        let (Some(pipeline), Some(l_screen), Some(sampler)) = (
            &self.pipeline,
            scene.assets.layouts.get(&POST_PROCESS_DEPTH_LAYOUT_UUID),
            &self.sampler,
        ) else {
            return;
        };

        // As the targets changes every frame, we need to create the bind group for each frame.
        let b_screen = renderer.device.create_bind_group(&BindGroupDescriptor {
            label: Some("screen_bind_group"),
            layout: l_screen,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(target.depth.as_ref().unwrap()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
        });

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("depth_view_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &target.color,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &b_screen, &[]);
            pass.draw(0..3, 0..1);
        }

        renderer.queue.submit(Some(encoder.finish()));
    }
}
>>>>>>> Stashed changes
