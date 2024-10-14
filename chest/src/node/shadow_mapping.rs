use std::{borrow::Cow, collections::HashMap};

use aurora_core::{
    render::{
        flow::RenderNode,
        resource::{DynamicGpuBuffer, GpuCamera, RenderMesh, RenderTargets, Vertex},
        scene::{
            ExtraBindGroupId, ExtraLayoutId, ExtraUniformBufferId, GpuScene, SamplerId, TextureId,
            TextureViewId,
        },
    },
    WgpuRenderer,
};
use naga_oil::compose::{Composer, NagaModuleDescriptor, ShaderDefValue};
use uuid::Uuid;
use wgpu::{
    vertex_attr_array, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BufferAddress, BufferBindingType,
    BufferUsages, CompareFunction, DepthBiasState, DepthStencilState, Extent3d, FilterMode,
    FragmentState, LoadOp, MultisampleState, Operations, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, SamplerBindingType,
    SamplerDescriptor, ShaderModuleDescriptor, ShaderSource, ShaderStages, StencilState, StoreOp,
    TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, VertexBufferLayout,
    VertexState, VertexStepMode,
};

use crate::util;

pub struct ShadowMapping {
    pub light_view_uniform: ExtraUniformBufferId,
    pub light_view_layout: ExtraLayoutId,
    pub light_view_bind_group: ExtraBindGroupId,
    pub directional_shadow_map: TextureId,
    pub directional_shadow_map_view: TextureViewId,
    pub point_shadow_map: TextureId,
    pub point_shadow_map_view: TextureViewId,
    pub shadow_map_sampler: SamplerId,

    pub shadow_maps_layout: ExtraLayoutId,
    pub shadow_maps_bind_group: ExtraBindGroupId,
}

pub const SHADOW_MAPPING: ShadowMapping = ShadowMapping {
    light_view_uniform: ExtraUniformBufferId(Uuid::from_u128(8794041105348641631856410231)),
    light_view_layout: ExtraLayoutId(Uuid::from_u128(7513015631563408941231)),
    light_view_bind_group: ExtraBindGroupId(Uuid::from_u128(123056463804784103210324847)),
    directional_shadow_map: TextureId(Uuid::from_u128(7861046541564897045132508964132)),
    directional_shadow_map_view: TextureViewId(Uuid::from_u128(10264856487964101541231456531)),
    point_shadow_map: TextureId(Uuid::from_u128(204153435154865423112313232)),
    point_shadow_map_view: TextureViewId(Uuid::from_u128(8974689406540351354897321563484)),
    shadow_map_sampler: SamplerId(Uuid::from_u128(8713416357854635486345415311523415)),
    shadow_maps_layout: ExtraLayoutId(Uuid::from_u128(9870130163543413521356876413)),
    shadow_maps_bind_group: ExtraBindGroupId(Uuid::from_u128(78974610032413605413136786)),
};

#[derive(Default)]
pub struct ShadowMappingNode {
    pipeline: Option<RenderPipeline>,
    directional_views: HashMap<Uuid, TextureViewId>,
    point_views: HashMap<Uuid, [TextureViewId; 6]>,
    offsets: Vec<u32>,
}

impl ShadowMappingNode {
    pub const CASCADE_COUNT: u32 = 3;
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
            include_str!("../shader/common/common_type.wgsl"),
            shader_defs.clone(),
        );
        util::add_shader_module(
            &mut composer,
            include_str!("../shader/common/common_binding.wgsl"),
            shader_defs.clone(),
        );
        let shader = composer
            .make_naga_module(NagaModuleDescriptor {
                source: include_str!("../shader/shadow/shadow_render.wgsl"),
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
