use std::{
    any::TypeId,
    borrow::Cow,
    collections::{hash_map::Entry, HashMap},
};

use aurora_core::{
    render::{
        flow::RenderNode,
        resource::{
            DynamicGpuBuffer, GpuCamera, RenderMesh, RenderTargets, Vertex, CAMERA_UUID,
            LIGHTS_BIND_GROUP_UUID, LIGHT_VIEW_UUID, POST_PROCESS_DEPTH_LAYOUT_UUID,
        },
        scene::GpuScene,
    },
    scene::{entity::Light, Scene},
    util::ext::TypeIdAsUuid,
    WgpuRenderer,
};
use naga_oil::compose::{
    ComposableModuleDescriptor, Composer, NagaModuleDescriptor, ShaderDefValue, ShaderLanguage,
    ShaderType,
};
use uuid::Uuid;
use wgpu::{
    vertex_attr_array, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BufferAddress, BufferBindingType,
    BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor, CompareFunction,
    DepthBiasState, DepthStencilState, Extent3d, Face, FilterMode, FragmentState, LoadOp,
    MultisampleState, Operations, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerDescriptor,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, StencilState, StoreOp, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
    TextureViewDimension, VertexBufferLayout, VertexState, VertexStepMode,
};

use crate::{
    material::PbrMaterial,
    resource::{ShadowMaps, SHADOW_MAPS},
    texture, util,
};
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
        _scene: &Scene,
        _gpu_scene: &mut GpuScene,
        _shader_defs: Option<HashMap<String, ShaderDefValue>>,
        target: &RenderTargets,
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
                    cache: None,
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
        _scene: &Scene,
        _gpu_scene: &mut GpuScene,
        _queue: &mut [RenderMesh],
        _target: &RenderTargets,
    ) {
    }

    fn draw(
        &self,
        renderer: &WgpuRenderer,
        _scene: &Scene,
        _gpu_scene: &GpuScene,
        _queue: &[RenderMesh],
        target: &RenderTargets,
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

pub const TONY_MC_MAPFACE_LUT: Uuid = Uuid::from_u128(7949841653150346834163056985041356);

#[derive(Default)]
pub struct PbrNode {
    mat_uuid: Uuid,
    pipeline: Option<RenderPipeline>,
}

impl RenderNode for PbrNode {
    fn build(
        &mut self,
        renderer: &WgpuRenderer,
        _scene: &Scene,
        gpu_scene: &mut GpuScene,
        shader_defs: Option<HashMap<String, ShaderDefValue>>,
        target: &RenderTargets,
    ) {
        gpu_scene.assets.textures.insert(
            TONY_MC_MAPFACE_LUT,
            texture::load_dds_texture(renderer, "chest/assets/luts/tony_mc_mapface.dds"),
        );

        self.mat_uuid = TypeId::of::<PbrMaterial>().to_uuid();

        let (l_camera, l_lights, l_material) = (
            &gpu_scene.assets.layouts[&CAMERA_UUID],
            &gpu_scene.assets.layouts[&LIGHTS_BIND_GROUP_UUID],
            &gpu_scene.assets.layouts[&self.mat_uuid],
        );

        let mut composer = Composer::default();
        util::add_shader_module(
            &mut composer,
            include_str!("shader/math.wgsl"),
            shader_defs.clone(),
        );
        util::add_shader_module(
            &mut composer,
            include_str!("shader/common/common_type.wgsl"),
            shader_defs.clone(),
        );
        util::add_shader_module(
            &mut composer,
            include_str!("shader/common/common_binding.wgsl"),
            shader_defs.clone(),
        );
        util::add_shader_module(
            &mut composer,
            include_str!("shader/pbr/pbr_type.wgsl"),
            shader_defs.clone(),
        );
        util::add_shader_module(
            &mut composer,
            include_str!("shader/pbr/pbr_binding.wgsl"),
            shader_defs.clone(),
        );
        util::add_shader_module(
            &mut composer,
            include_str!("shader/pbr/pbr_function.wgsl"),
            shader_defs.clone(),
        );
        util::add_shader_module(
            &mut composer,
            include_str!("shader/tonemapping.wgsl"),
            shader_defs.clone(),
        );
        util::add_shader_module(
            &mut composer,
            include_str!("shader/pbr/pbr.wgsl"),
            shader_defs.clone(),
        );

        let mut shader_defs = shader_defs.unwrap_or_default();
        shader_defs.extend([
            ("LUT_TEX_BINDING".to_string(), ShaderDefValue::UInt(4)),
            ("LUT_SAMPLER_BINDING".to_string(), ShaderDefValue::UInt(5)),
        ]);
        let shader = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("shader/pbr/pbr.wgsl"),
                shader_defs,
                ..Default::default()
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
                    cache: None,
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
                                2 => Float32x2,
                                // Tangent
                                3 => Float32x4,
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
        );
    }

    fn prepare(
        &mut self,
        renderer: &WgpuRenderer,
        _scene: &Scene,
        gpu_scene: &mut GpuScene,
        queue: &mut [RenderMesh],
        _target: &RenderTargets,
    ) {
        match gpu_scene.assets.buffers.entry(self.mat_uuid) {
            Entry::Occupied(mut e) => e.get_mut().clear(),
            Entry::Vacant(e) => {
                e.insert(DynamicGpuBuffer::new(BufferUsages::UNIFORM));
            }
        }

        queue
            .iter_mut()
            .filter_map(|sm| gpu_scene.materials.get(&sm.mesh.material).map(|m| (m, sm)))
            .for_each(|(material, mesh)| {
                mesh.offset = Some(material.prepare(renderer, &mut gpu_scene.assets));
            });

        gpu_scene
            .assets
            .buffers
            .get_mut(&self.mat_uuid)
            .unwrap()
            .write(&renderer.device, &renderer.queue);

        queue
            .iter_mut()
            .filter_map(|sm| gpu_scene.materials.get(&sm.mesh.material).map(|m| (m, sm)))
            .for_each(|(material, mesh)| {
                material.create_bind_group(renderer, &mut gpu_scene.assets, mesh.mesh.material);
            });
    }

    fn draw(
        &self,
        renderer: &WgpuRenderer,
        _scene: &Scene,
        gpu_scene: &GpuScene,
        queue: &[RenderMesh],
        target: &RenderTargets,
    ) {
        let assets = &gpu_scene.assets;

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
                let (Some(b_material), Some(vertices)) = (
                    assets.bind_groups.get(&mesh.mesh.material),
                    assets.buffers.get(&mesh.mesh.mesh),
                ) else {
                    continue;
                };

                pass.set_bind_group(2, b_material, &[mesh.offset.unwrap()]);
                pass.set_vertex_buffer(0, vertices.buffer().unwrap().slice(..));
                pass.draw(0..vertices.len::<Vertex>().unwrap() as u32, 0..1);
            }
        }

        renderer.queue.submit(Some(encoder.finish()));
    }
}

#[derive(Default)]
pub struct DepthViewNode {
    pipeline: Option<RenderPipeline>,
    sampler: Option<Sampler>,
}

impl RenderNode for DepthViewNode {
    fn build(
        &mut self,
        renderer: &WgpuRenderer,
        _scene: &Scene,
        gpu_scene: &mut GpuScene,
        _shader_defs: Option<HashMap<String, ShaderDefValue>>,
        target: &RenderTargets,
    ) {
        let Some(l_post_process) = gpu_scene
            .assets
            .layouts
            .get(&POST_PROCESS_DEPTH_LAYOUT_UUID)
        else {
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
                    cache: None,
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
        _scene: &Scene,
        _gpu_scene: &mut GpuScene,
        _queue: &mut [RenderMesh],
        _target: &RenderTargets,
    ) {
    }

    fn draw(
        &self,
        renderer: &WgpuRenderer,
        _scene: &Scene,
        gpu_scene: &GpuScene,
        _queue: &[RenderMesh],
        target: &RenderTargets,
    ) {
        let mut encoder = renderer
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        let (Some(pipeline), Some(l_screen), Some(sampler)) = (
            &self.pipeline,
            gpu_scene
                .assets
                .layouts
                .get(&POST_PROCESS_DEPTH_LAYOUT_UUID),
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
                    resource: BindingResource::TextureView(
                        &SHADOW_MAPS
                            .lock()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .directional_shadow_map
                            .create_view(&TextureViewDescriptor {
                                format: Some(TextureFormat::Depth32Float),
                                dimension: Some(TextureViewDimension::D2),
                                base_array_layer: 0,
                                array_layer_count: Some(1),
                                ..Default::default()
                            }),
                    ),
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

#[derive(Default)]
pub struct ShadowMappingNode {
    pipeline: Option<RenderPipeline>,
    shadow_map_views: HashMap<Uuid, TextureView>,
}

impl RenderNode for ShadowMappingNode {
    fn build(
        &mut self,
        renderer: &WgpuRenderer,
        _scene: &Scene,
        gpu_scene: &mut GpuScene,
        shader_defs: Option<HashMap<String, ShaderDefValue>>,
        target: &RenderTargets,
    ) {
        let mut composer = Composer::default();
        util::add_shader_module(
            &mut composer,
            include_str!("shader/common/common_type.wgsl"),
            shader_defs.clone(),
        );
        util::add_shader_module(
            &mut composer,
            include_str!("shader/common/common_binding.wgsl"),
            shader_defs.clone(),
        );
        let shader = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("shader/shadow/shadow_render.wgsl"),
                shader_defs: shader_defs.unwrap_or_default(),
                ..Default::default()
            })
            .unwrap();

        let module = renderer
            .device
            .create_shader_module(ShaderModuleDescriptor {
                label: Some("shadow_render_shader"),
                source: ShaderSource::Naga(Cow::Owned(shader)),
            });

        let light_view_layout =
            renderer
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some(<GpuCamera as encase::ShaderType>::min_size()),
                        },
                        count: None,
                    }],
                });

        let layout = renderer
            .device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("shadow_mapping_shader"),
                bind_group_layouts: &[&light_view_layout],
                push_constant_ranges: &[],
            });

        gpu_scene
            .assets
            .layouts
            .insert(LIGHT_VIEW_UUID, light_view_layout);

        self.pipeline = Some(
            renderer
                .device
                .create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some("shadow_mapping_pipeline"),
                    layout: Some(&layout),
                    cache: None,
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
                                2 => Float32x2,
                                // Tangent
                                3 => Float32x4,
                            ],
                        }],
                    },
                    multisample: MultisampleState::default(),
                    fragment: Some(FragmentState {
                        module: &module,
                        entry_point: "fragment",
                        compilation_options: PipelineCompilationOptions::default(),
                        targets: &[None],
                    }),
                    // fragment: None,
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
        );

        let directional_shadow_map = renderer.device.create_texture(&TextureDescriptor {
            label: Some("directional_shadow_map"),
            size: Extent3d {
                width: 1024,
                height: 1024,
                depth_or_array_layers: gpu_scene.light_counter.directional_lights.max(1),
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let point_shadow_map = renderer.device.create_texture(&TextureDescriptor {
            label: Some("point_shadow_map"),
            size: Extent3d {
                width: 1024,
                height: 1024,
                depth_or_array_layers: (gpu_scene.light_counter.omni_lights * 6).max(1),
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let directional_shadow_sampler = renderer.device.create_sampler(&SamplerDescriptor {
            label: Some("directional_shadow_sampler"),
            compare: Some(CompareFunction::Less),
            ..Default::default()
        });

        let point_shadow_sampler = renderer.device.create_sampler(&SamplerDescriptor {
            label: Some("point_shadow_sampler"),
            compare: Some(CompareFunction::Less),
            ..Default::default()
        });

        SHADOW_MAPS.lock().unwrap().replace(ShadowMaps {
            directional_shadow_map,
            directional_shadow_sampler,
            point_shadow_map,
            point_shadow_sampler,
        });
    }

    fn prepare(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &Scene,
        gpu_scene: &mut GpuScene,
        _queue: &mut [RenderMesh],
        _target: &RenderTargets,
    ) {
        self.shadow_map_views.clear();
        let shadow_maps = SHADOW_MAPS.lock();
        let shadow_maps = shadow_maps.as_ref().unwrap().as_ref().unwrap();

        let mut directional_index = 0;
        let mut point_index = 0;

        let mut directional_desc = TextureViewDescriptor {
            label: Some("directional_shadow_map_view"),
            format: Some(TextureFormat::Depth32Float),
            dimension: Some(TextureViewDimension::D2),
            base_array_layer: 0,
            array_layer_count: Some(1),
            ..Default::default()
        };

        let mut point_desc = TextureViewDescriptor {
            label: Some("point_shadow_map_view"),
            format: Some(TextureFormat::Depth32Float),
            dimension: Some(TextureViewDimension::Cube),
            base_array_layer: 0,
            array_layer_count: Some(6),
            ..Default::default()
        };

        for (id, light) in &scene.lights {
            match light {
                Light::Directional(_) => {
                    directional_desc.base_array_layer = directional_index;
                    self.shadow_map_views.insert(
                        *id,
                        shadow_maps
                            .directional_shadow_map
                            .create_view(&directional_desc),
                    );
                    directional_index += 1;
                }
                Light::Point(_) | Light::Spot(_) => {
                    point_desc.base_array_layer = point_index * 6;
                    self.shadow_map_views
                        .insert(*id, shadow_maps.point_shadow_map.create_view(&point_desc));
                    point_index += 1;
                }
            }
        }

        gpu_scene.assets.bind_groups.insert(
            LIGHT_VIEW_UUID,
            renderer.device.create_bind_group(&BindGroupDescriptor {
                label: Some("light_view_bind_group"),
                layout: gpu_scene.assets.layouts.get(&LIGHT_VIEW_UUID).unwrap(),
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: gpu_scene
                        .assets
                        .buffers
                        .get(&LIGHT_VIEW_UUID)
                        .unwrap()
                        .entire_binding()
                        .unwrap(),
                }],
            }),
        );
    }

    fn draw(
        &self,
        renderer: &WgpuRenderer,
        scene: &Scene,
        gpu_scene: &GpuScene,
        queue: &[RenderMesh],
        _target: &RenderTargets,
    ) {
        let assets = &gpu_scene.assets;

        let Some(light_view_bind_groups) = assets.bind_groups.get(&LIGHT_VIEW_UUID) else {
            dbg!();
            return;
        };

        let mut encoder = renderer.device.create_command_encoder(&Default::default());
        for (id, _) in &scene.lights {
            dbg!();
            let Some(pipeline) = &self.pipeline else {
                return;
            };

            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("shadow_pass"),
                color_attachments: &[None],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: self.shadow_map_views.get(id).unwrap(),
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            dbg!();

            pass.set_pipeline(pipeline);
            pass.set_bind_group(
                0,
                light_view_bind_groups,
                &[*assets.offsets.get(id).unwrap()],
            );

            for mesh in queue {
                let Some(vertices) = assets.buffers.get(&mesh.mesh.mesh) else {
                    return;
                };

                pass.set_vertex_buffer(0, vertices.buffer().unwrap().slice(..));
                pass.draw(0..vertices.len::<Vertex>().unwrap() as u32, 0..1);
            }
        }
        renderer.queue.submit([encoder.finish()]);
    }
}
