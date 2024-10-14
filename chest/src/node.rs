use std::{
    any::TypeId,
    borrow::Cow,
    collections::{hash_map::Entry, HashMap},
};

use aurora_core::{
    render::{
        flow::RenderNode,
        mesh::CreateBindGroupLayout,
        resource::{
            DynamicGpuBuffer, GpuCamera, RenderMesh, RenderTargets, Vertex,
            POST_PROCESS_DEPTH_LAYOUT_UUID,
        },
        scene::{GpuScene, MaterialTypeId, TextureId, TextureViewId},
    },
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
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType,
    SamplerDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, StencilState, StoreOp,
    TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, VertexBufferLayout,
    VertexState, VertexStepMode,
};

use crate::{
    material::{PbrMaterial, PbrMaterialUniform},
    resource::SHADOW_MAPPING,
    texture, util,
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
        _scene: &mut GpuScene,
        _queue: &mut [RenderMesh],
        _target: &RenderTargets,
    ) {
    }

    fn draw(
        &self,
        renderer: &WgpuRenderer,
        _scene: &mut GpuScene,
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

pub const TONY_MC_MAPFACE_LUT: TextureId =
    TextureId(Uuid::from_u128(7949841653150346834163056985041356));

#[derive(Default)]
pub struct PbrNode {
    mat_uuid: MaterialTypeId,
    pipeline: Option<RenderPipeline>,
}

impl RenderNode for PbrNode {
    fn build(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        shader_defs: Option<HashMap<String, ShaderDefValue>>,
        target: &RenderTargets,
    ) {
        scene.assets.textures.insert(
            TONY_MC_MAPFACE_LUT,
            texture::load_dds_texture(renderer, "chest/assets/luts/tony_mc_mapface.dds"),
        );

        self.mat_uuid = MaterialTypeId(TypeId::of::<PbrMaterial>().to_uuid());
        PbrMaterial::create_layout(renderer, &mut scene.assets);

        let Some(l_shadow_map) = scene
            .assets
            .extra_layouts
            .get(&SHADOW_MAPPING.shadow_maps_layout)
        else {
            return;
        };

        let (l_camera, l_lights, l_material) = (
            scene.assets.common_layout.as_ref().unwrap(),
            scene.assets.lights_layout.as_ref().unwrap(),
            &scene.assets.material_layouts[&self.mat_uuid],
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
            include_str!("shader/shadow/shadow_mapping.wgsl"),
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
                bind_group_layouts: &[&l_camera, &l_lights, &l_material, &l_shadow_map],
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
        scene: &mut GpuScene,
        queue: &mut [RenderMesh],
        _target: &RenderTargets,
    ) {
        match scene.assets.material_uniforms.entry(self.mat_uuid) {
            Entry::Occupied(mut e) => e.get_mut().clear(),
            Entry::Vacant(e) => {
                e.insert(DynamicGpuBuffer::new(BufferUsages::UNIFORM));
            }
        }

        queue
            .iter_mut()
            .filter_map(|sm| {
                scene
                    .original
                    .materials
                    .get(&sm.mesh.material)
                    .map(|m| (m, sm))
            })
            .for_each(|(material, mesh)| {
                mesh.offset = Some(material.prepare(renderer, &mut scene.assets));
            });

        scene
            .assets
            .material_uniforms
            .get_mut(&self.mat_uuid)
            .unwrap()
            .write::<PbrMaterialUniform>(&renderer.device, &renderer.queue);

        queue
            .iter_mut()
            .filter_map(|sm| {
                scene
                    .original
                    .materials
                    .get(&sm.mesh.material)
                    .map(|m| (m, sm))
            })
            .for_each(|(material, mesh)| {
                material.create_bind_group(renderer, &mut scene.assets, mesh.mesh.material);
            });
    }

    fn draw(
        &self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        queue: &[RenderMesh],
        target: &RenderTargets,
    ) {
        let assets = &scene.assets;

        let mut encoder = renderer
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        let (Some(b_camera), Some(b_lights), Some(b_shadow_maps)) = (
            &assets.common_bind_group,
            &assets.light_bind_group,
            assets
                .extra_bind_groups
                .get(&SHADOW_MAPPING.shadow_maps_bind_group),
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
            pass.set_bind_group(3, &b_shadow_maps, &[]);

            for mesh in queue {
                let (Some(b_material), Some((vertices, count))) = (
                    assets.material_bind_groups.get(&mesh.mesh.material),
                    assets.vertex_buffers.get(&mesh.mesh.mesh),
                ) else {
                    continue;
                };

                pass.set_bind_group(2, b_material, &[mesh.offset.unwrap()]);
                pass.set_vertex_buffer(0, vertices.buffer().unwrap().slice(..));
                pass.draw(0..*count, 0..1);
            }
        }

        renderer.queue.submit([encoder.finish()]);
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
        scene: &mut GpuScene,
        _shader_defs: Option<HashMap<String, ShaderDefValue>>,
        target: &RenderTargets,
    ) {
        let Some(l_post_process) = scene
            .assets
            .material_layouts
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
        _scene: &mut GpuScene,
        _queue: &mut [RenderMesh],
        _target: &RenderTargets,
    ) {
    }

    fn draw(
        &self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        _queue: &[RenderMesh],
        target: &RenderTargets,
    ) {
        let mut encoder = renderer
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        let (Some(pipeline), Some(l_screen), Some(sampler)) = (
            &self.pipeline,
            scene
                .assets
                .material_layouts
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
                        &scene.assets.textures[&SHADOW_MAPPING.directional_shadow_map].create_view(
                            &TextureViewDescriptor {
                                format: Some(TextureFormat::Depth32Float),
                                dimension: Some(TextureViewDimension::D2),
                                base_array_layer: 0,
                                array_layer_count: Some(1),
                                ..Default::default()
                            },
                        ),
                        // target.depth.as_ref().unwrap(),
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
    directional_views: HashMap<Uuid, TextureViewId>,
    point_views: HashMap<Uuid, [TextureViewId; 6]>,
    offsets: Vec<u32>,
}

impl RenderNode for ShadowMappingNode {
    fn build(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
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
                        visibility: ShaderStages::VERTEX_FRAGMENT,
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

        scene
            .assets
            .extra_layouts
            .insert(SHADOW_MAPPING.light_view_layout, light_view_layout);

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
                        // cull_mode: Some(Face::Back),
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
                depth_or_array_layers: (scene.original.directional_lights.len() as u32).max(1),
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let directional_shadow_map_view =
            directional_shadow_map.create_view(&TextureViewDescriptor {
                label: Some("directional_shadow_map_view"),
                dimension: Some(TextureViewDimension::D2Array),
                ..Default::default()
            });

        let point_shadow_map = renderer.device.create_texture(&TextureDescriptor {
            label: Some("point_shadow_map"),
            size: Extent3d {
                width: 512,
                height: 512,
                depth_or_array_layers: ((scene.original.point_lights.len() as u32
                    + scene.original.spot_lights.len() as u32)
                    * 6)
                .max(6),
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let point_shadow_map_view = point_shadow_map.create_view(&TextureViewDescriptor {
            label: Some("point_shadow_map_view"),
            dimension: Some(TextureViewDimension::CubeArray),
            aspect: TextureAspect::DepthOnly,
            ..Default::default()
        });

        let shadow_map_sampler = renderer.device.create_sampler(&SamplerDescriptor {
            label: Some("shadow_map_sampler"),
            compare: Some(CompareFunction::LessEqual),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        let shadow_maps_layout =
            renderer
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("shadow_layout"),
                    entries: &[
                        // Light Views
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: Some(
                                    <GpuCamera as encase::ShaderType>::min_size(),
                                ),
                            },
                            count: None,
                        },
                        // Shadow Map Sampler
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Sampler(SamplerBindingType::Comparison),
                            count: None,
                        },
                        // Directional Light Shaodow Maps
                        BindGroupLayoutEntry {
                            binding: 2,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Texture {
                                sample_type: TextureSampleType::Depth,
                                view_dimension: TextureViewDimension::D2Array,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // Point Light Shaodow Maps
                        BindGroupLayoutEntry {
                            binding: 3,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Texture {
                                sample_type: TextureSampleType::Depth,
                                view_dimension: TextureViewDimension::CubeArray,
                                multisampled: false,
                            },
                            count: None,
                        },
                    ],
                });

        scene
            .assets
            .extra_layouts
            .insert(SHADOW_MAPPING.shadow_maps_layout, shadow_maps_layout);
        scene
            .assets
            .samplers
            .insert(SHADOW_MAPPING.shadow_map_sampler, shadow_map_sampler);
        scene.assets.textures.insert(
            SHADOW_MAPPING.directional_shadow_map,
            directional_shadow_map,
        );
        scene.assets.texture_views.insert(
            SHADOW_MAPPING.directional_shadow_map_view,
            directional_shadow_map_view,
        );
        scene
            .assets
            .textures
            .insert(SHADOW_MAPPING.point_shadow_map, point_shadow_map);
        scene
            .assets
            .texture_views
            .insert(SHADOW_MAPPING.point_shadow_map_view, point_shadow_map_view);
    }

    fn prepare(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        _queue: &mut [RenderMesh],
        _target: &RenderTargets,
    ) {
        let mut directional_index = 0;
        let mut point_index = 0;

        let mut directional_desc = TextureViewDescriptor {
            label: Some("directional_shadow_map_render_view"),
            format: Some(TextureFormat::Depth32Float),
            dimension: Some(TextureViewDimension::D2),
            aspect: TextureAspect::DepthOnly,
            base_array_layer: 0,
            array_layer_count: Some(1),
            ..Default::default()
        };

        let mut point_desc = TextureViewDescriptor {
            label: Some("point_shadow_map_render_view"),
            format: Some(TextureFormat::Depth32Float),
            dimension: Some(TextureViewDimension::D2),
            aspect: TextureAspect::DepthOnly,
            base_array_layer: 0,
            array_layer_count: Some(1),
            ..Default::default()
        };

        let mut bf_light_view =
            DynamicGpuBuffer::new(BufferUsages::UNIFORM | BufferUsages::STORAGE);

        let directional_shadow_maps =
            &scene.assets.textures[&SHADOW_MAPPING.directional_shadow_map];
        let point_shadow_maps = &scene.assets.textures[&SHADOW_MAPPING.point_shadow_map];

        for (id, light) in &scene.original.directional_lights {
            directional_desc.base_array_layer = directional_index;
            let texture_view_id = TextureViewId(Uuid::new_v4());

            self.directional_views.insert(*id, texture_view_id);
            scene.assets.texture_views.insert(
                texture_view_id,
                directional_shadow_maps.create_view(&directional_desc),
            );

            self.offsets.push(bf_light_view.push(&light.light_view()));
            directional_index += 1;
        }

        for (id, light) in &scene.original.point_lights {
            let light_views = light.light_view();
            let mut texture_views = [TextureViewId::default(); 6];

            for i_face in 0..6 {
                point_desc.base_array_layer = point_index * 6 + i_face as u32;
                let texture_view_id = TextureViewId(Uuid::new_v4());
                texture_views[i_face] = texture_view_id;
                scene
                    .assets
                    .texture_views
                    .insert(texture_view_id, point_shadow_maps.create_view(&point_desc));
                self.offsets.push(bf_light_view.push(&light_views[i_face]));
            }

            self.point_views.insert(*id, texture_views);
            point_index += 1;
        }

        for (id, light) in &scene.original.spot_lights {
            let light_views = light.light_view();
            let mut texture_views = [TextureViewId::default(); 6];

            for i_face in 0..6 {
                point_desc.base_array_layer = point_index * 6 + i_face as u32;
                let texture_view_id = TextureViewId(Uuid::new_v4());
                texture_views[i_face] = texture_view_id;
                scene
                    .assets
                    .texture_views
                    .insert(texture_view_id, point_shadow_maps.create_view(&point_desc));
                self.offsets.push(bf_light_view.push(&light_views[i_face]));
            }

            self.point_views.insert(*id, texture_views);
            point_index += 1;
        }

        bf_light_view.write::<GpuCamera>(&renderer.device, &renderer.queue);
        scene
            .assets
            .extra_uniforms
            .insert(SHADOW_MAPPING.light_view_uniform, bf_light_view);

        scene.assets.extra_bind_groups.insert(
            SHADOW_MAPPING.light_view_bind_group,
            renderer.device.create_bind_group(&BindGroupDescriptor {
                label: Some("light_view_bind_group"),
                layout: &scene.assets.extra_layouts[&SHADOW_MAPPING.light_view_layout],
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: scene.assets.extra_uniforms[&SHADOW_MAPPING.light_view_uniform]
                        .binding::<GpuCamera>()
                        .unwrap(),
                }],
            }),
        );

        let shadow_map_bind_group = renderer.device.create_bind_group(&BindGroupDescriptor {
            label: Some("shadow_map_bind_group"),
            layout: &scene.assets.extra_layouts[&SHADOW_MAPPING.shadow_maps_layout],
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: scene.assets.extra_uniforms[&SHADOW_MAPPING.light_view_uniform]
                        .entire_binding()
                        .unwrap(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(
                        &scene.assets.samplers[&SHADOW_MAPPING.shadow_map_sampler],
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(
                        &scene.assets.texture_views[&SHADOW_MAPPING.directional_shadow_map_view],
                    ),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(
                        &scene.assets.texture_views[&SHADOW_MAPPING.point_shadow_map_view],
                    ),
                },
            ],
        });

        scene
            .assets
            .extra_bind_groups
            .insert(SHADOW_MAPPING.shadow_maps_bind_group, shadow_map_bind_group);
    }

    fn draw(
        &self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        queue: &[RenderMesh],
        _target: &RenderTargets,
    ) {
        let assets = &scene.assets;

        let Some(light_view_bind_groups) = assets
            .extra_bind_groups
            .get(&SHADOW_MAPPING.light_view_bind_group)
        else {
            return;
        };

        let mut view_index = 0;
        let mut encoder = renderer.device.create_command_encoder(&Default::default());
        let Some(pipeline) = &self.pipeline else {
            return;
        };

        let mut _draw = |depth_view: &TextureView| {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("shadow_pass"),
                color_attachments: &[None],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, light_view_bind_groups, &[self.offsets[view_index]]);

            for mesh in queue {
                let Some((vertices, count)) = assets.vertex_buffers.get(&mesh.mesh.mesh) else {
                    return;
                };

                pass.set_vertex_buffer(0, vertices.buffer().unwrap().slice(..));
                pass.draw(0..*count, 0..1);
            }

            view_index += 1;
        };

        for id in scene.original.directional_lights.keys() {
            let texture_view_id = &self.directional_views[id];
            _draw(&scene.assets.texture_views[&texture_view_id]);
        }

        for id in scene.original.point_lights.keys() {
            for texture_view_id in &self.point_views[id] {
                _draw(&scene.assets.texture_views[&texture_view_id]);
            }
        }

        renderer.queue.submit([encoder.finish()]);
    }
}
