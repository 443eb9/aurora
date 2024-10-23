use std::{borrow::Cow, collections::HashMap};

use aurora_core::{
    render::{
        flow::RenderNode,
        helper::{CameraProjection, Transform},
        resource::{DynamicGpuBuffer, GpuCamera, RenderMesh, RenderTargets, Vertex},
        scene::{
            ExtraBindGroupId, ExtraBufferId, ExtraLayoutId, GpuScene, SamplerId, TextureId,
            TextureViewId,
        },
        ShaderDefEnum,
    },
    WgpuRenderer,
};
use encase::ShaderType;
use glam::{Mat4, Vec2, Vec3};
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

use crate::{
    shader_defs::ShadowFilter,
    util::{self, frustum_slice, Aabb},
};

#[derive(ShaderType)]
pub struct ShadowMappingConfig {
    pub dir_map_resolution: u32,
    pub point_map_resolution: u32,
    pub samples: u32,
    pub pcf_radius: f32,
    pub pcss_radius: f32,
}

pub struct ShadowMapping {
    pub light_views: ExtraBufferId,
    pub cascade_views: ExtraBufferId,
    pub point_light_views: ExtraBufferId,
    pub poisson_disk: ExtraBufferId,
    pub config: ExtraBufferId,

    pub directional_shadow_map: TextureId,
    pub directional_shadow_map_view: TextureViewId,
    pub point_shadow_map: TextureId,
    pub point_shadow_map_view: TextureViewId,
    pub shadow_map_sampler: SamplerId,
    pub shadow_texture_sampler: SamplerId,

    pub shadow_maps_layout: ExtraLayoutId,
    pub light_view_layout: ExtraLayoutId,

    pub shadow_maps_bind_group: ExtraBindGroupId,
    pub light_views_bind_group: ExtraBindGroupId,
}

pub const SHADOW_MAPPING: ShadowMapping = ShadowMapping {
    light_views: ExtraBufferId(Uuid::from_u128(89413211065410340136548487101523115648)),
    cascade_views: ExtraBufferId(Uuid::from_u128(894132906465410168465132984653696845)),
    point_light_views: ExtraBufferId(Uuid::from_u128(8794041105348641631856410231)),
    poisson_disk: ExtraBufferId(Uuid::from_u128(1687846160641318676894156310604693)),
    config: ExtraBufferId(Uuid::from_u128(1354687841323006814572453187684531684)),

    directional_shadow_map: TextureId(Uuid::from_u128(7861046541564897045132508964132)),
    directional_shadow_map_view: TextureViewId(Uuid::from_u128(10264856487964101541231456531)),
    point_shadow_map: TextureId(Uuid::from_u128(204153435154865423112313232)),
    point_shadow_map_view: TextureViewId(Uuid::from_u128(8974689406540351354897321563484)),
    shadow_map_sampler: SamplerId(Uuid::from_u128(8713416357854635486345415311523415)),
    shadow_texture_sampler: SamplerId(Uuid::from_u128(78946512367469845123501009864354)),

    light_view_layout: ExtraLayoutId(Uuid::from_u128(7513015631563408941231)),
    shadow_maps_layout: ExtraLayoutId(Uuid::from_u128(9870130163543413521356876413)),

    shadow_maps_bind_group: ExtraBindGroupId(Uuid::from_u128(78974610032413605413136786)),
    light_views_bind_group: ExtraBindGroupId(Uuid::from_u128(135648640640653130645120465123)),
};

#[derive(Default)]
pub struct ShadowMappingNode {
    pipeline: Option<RenderPipeline>,
    directional_views: HashMap<Uuid, [TextureViewId; Self::CASCADE_COUNT]>,
    point_views: HashMap<Uuid, [TextureViewId; 6]>,
    offsets: Vec<u32>,
}

impl ShadowMappingNode {
    pub const CASCADE_COUNT: usize = 2;
    pub const CONFIG: ShadowMappingConfig = ShadowMappingConfig {
        dir_map_resolution: 2048,
        point_map_resolution: 512,
        samples: 64,
        pcf_radius: 4.,
        pcss_radius: 4.,
    };

    pub fn calculate_cascade_view(
        camera_transform: Transform,
        camera_proj_slice: CameraProjection,
        light_dir: Vec3,
    ) -> GpuCamera {
        let view_proj =
            camera_proj_slice.compute_matrix() * camera_transform.compute_matrix().inverse();
        // Frustum corners in world space.
        let mut frustum_corners = util::calculate_frustum_corners(view_proj);

        // The transform of this cascade should at center of that frustum.
        let center = frustum_corners.into_iter().reduce(|v, c| v + c).unwrap()
            / frustum_corners.len() as f32;

        // And looking at the light_dir.
        // As we are having the inverse direction, which is only use for light calculation,
        // inverse it back.
        let cascade_view = Mat4::look_to_rh(center, -light_dir, Vec3::Y);

        // Convert frustum into cascade view space.
        frustum_corners
            .iter_mut()
            .for_each(|c| *c = (cascade_view * c.extend(1.)).truncate());

        // Calculate the bounding box of the frustum in cascade view space.
        let cascade_proj_aabb = frustum_corners.into_iter().fold(
            Aabb {
                min: Vec3::MAX,
                max: Vec3::MIN,
            },
            |mut aabb, c| {
                aabb.min = aabb.min.min(c);
                aabb.max = aabb.max.max(c);
                aabb
            },
        );
        let half_aabb_size = (cascade_proj_aabb.max - cascade_proj_aabb.min) * 0.5;

        let cascade_proj = Mat4::orthographic_rh(
            -half_aabb_size.x,
            half_aabb_size.x,
            -half_aabb_size.y,
            half_aabb_size.y,
            -half_aabb_size.z * 10.,
            half_aabb_size.z,
        );

        GpuCamera {
            view: cascade_view,
            proj: cascade_proj,
            position_ws: center,
            // SPECIAL USE CASE!!
            exposure: match camera_proj_slice {
                CameraProjection::Perspective(proj) => proj.near,
                CameraProjection::Orthographic(proj) => proj.near,
            },
        }
    }
}

impl RenderNode for ShadowMappingNode {
    fn require_shader_defs(&self, shader_defs: &mut HashMap<String, ShaderDefValue>) {
        shader_defs.extend([
            (
                "SHADOW_CASCADES".to_owned(),
                ShaderDefValue::UInt(Self::CASCADE_COUNT as u32),
            ),
            ("SHOW_CASCADES".to_owned(), ShaderDefValue::Bool(true)),
            // ShadowFilter::PCF.to_def(),
        ]);
    }

    fn build(
        &mut self,
        renderer: &WgpuRenderer,
        GpuScene {
            original,
            assets,
            static_meshes: _,
        }: &mut GpuScene,
        shader_defs: HashMap<String, ShaderDefValue>,
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
                shader_defs,
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

        assets
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

        let cascade_directional_shadow_map = renderer.device.create_texture(&TextureDescriptor {
            label: Some("cascade_directional_shadow_map"),
            size: Extent3d {
                width: Self::CONFIG.dir_map_resolution,
                height: Self::CONFIG.dir_map_resolution,
                depth_or_array_layers: ((original.directional_lights.len() * Self::CASCADE_COUNT)
                    as u32)
                    .max(1),
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let cascade_directional_shadow_map_view =
            cascade_directional_shadow_map.create_view(&TextureViewDescriptor {
                label: Some("cascade_directional_shadow_map_view"),
                dimension: Some(TextureViewDimension::D2Array),
                ..Default::default()
            });

        let point_shadow_map = renderer.device.create_texture(&TextureDescriptor {
            label: Some("point_shadow_map"),
            size: Extent3d {
                width: Self::CONFIG.point_map_resolution,
                height: Self::CONFIG.point_map_resolution,
                depth_or_array_layers: ((original.point_lights.len() as u32
                    + original.spot_lights.len() as u32)
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

        let shadow_texture_sampler = renderer.device.create_sampler(&SamplerDescriptor {
            label: Some("shadow_texture_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        let shadow_maps_layout =
            renderer
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("shadow_maps_layout"),
                    entries: &[
                        // Cascade Views
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
                        // Point Light Views
                        BindGroupLayoutEntry {
                            binding: 1,
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
                            binding: 2,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Sampler(SamplerBindingType::Comparison),
                            count: None,
                        },
                        // Shadow Texture Sampler
                        BindGroupLayoutEntry {
                            binding: 3,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Sampler(SamplerBindingType::Filtering),
                            count: None,
                        },
                        // Directional Light Shaodow Maps
                        BindGroupLayoutEntry {
                            binding: 4,
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
                            binding: 5,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Texture {
                                sample_type: TextureSampleType::Depth,
                                view_dimension: TextureViewDimension::CubeArray,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // Poisson Disk
                        BindGroupLayoutEntry {
                            binding: 6,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: Some(<Vec2 as encase::ShaderType>::min_size()),
                            },
                            count: None,
                        },
                        // Config
                        BindGroupLayoutEntry {
                            binding: 7,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: Some(
                                    <ShadowMappingConfig as encase::ShaderType>::min_size(),
                                ),
                            },
                            count: None,
                        },
                    ],
                });

        assets
            .extra_layouts
            .insert(SHADOW_MAPPING.shadow_maps_layout, shadow_maps_layout);
        assets
            .samplers
            .insert(SHADOW_MAPPING.shadow_map_sampler, shadow_map_sampler);
        assets.samplers.insert(
            SHADOW_MAPPING.shadow_texture_sampler,
            shadow_texture_sampler,
        );
        assets.textures.insert(
            SHADOW_MAPPING.directional_shadow_map,
            cascade_directional_shadow_map,
        );
        assets.texture_views.insert(
            SHADOW_MAPPING.directional_shadow_map_view,
            cascade_directional_shadow_map_view,
        );
        assets
            .textures
            .insert(SHADOW_MAPPING.point_shadow_map, point_shadow_map);
        assets
            .texture_views
            .insert(SHADOW_MAPPING.point_shadow_map_view, point_shadow_map_view);

        let mut bf_poisson_disk = DynamicGpuBuffer::new(BufferUsages::STORAGE);
        let mut raw_poisson_disk = Vec::with_capacity(Self::CONFIG.samples as usize * 2);
        fast_poisson::Poisson2D::new()
            .into_iter()
            .for_each(|mut x| {
                x[0] = x[0] * 2. - 1.;
                x[1] = x[1] * 2. - 1.;
                raw_poisson_disk.extend_from_slice(bytemuck::bytes_of(&x));
            });

        bf_poisson_disk.set(raw_poisson_disk);
        bf_poisson_disk.write::<Vec2>(&renderer.device, &renderer.queue);
        assets
            .extra_buffers
            .insert(SHADOW_MAPPING.poisson_disk, bf_poisson_disk);

        let mut bf_config = DynamicGpuBuffer::new(BufferUsages::UNIFORM);
        bf_config.push(&Self::CONFIG);
        bf_config.write::<ShadowMappingConfig>(&renderer.device, &renderer.queue);
        assets
            .extra_buffers
            .insert(SHADOW_MAPPING.config, bf_config);
    }

    fn prepare(
        &mut self,
        renderer: &WgpuRenderer,
        GpuScene {
            original,
            assets,
            static_meshes: _,
        }: &mut GpuScene,
        _queue: &mut [RenderMesh],
        _target: &RenderTargets,
    ) {
        let mut directional_index = 0;
        let mut point_index = 0;

        let mut cascade_directional_desc = TextureViewDescriptor {
            label: Some("cascade_directional_shadow_map_render_view"),
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

        let mut raw_cascade_views = Vec::new();
        let mut bf_point_light_view = DynamicGpuBuffer::new(BufferUsages::STORAGE);
        let mut bf_light_views = DynamicGpuBuffer::new(BufferUsages::UNIFORM);

        let directional_shadow_maps = &assets.textures[&SHADOW_MAPPING.directional_shadow_map];
        let point_shadow_maps = &assets.textures[&SHADOW_MAPPING.point_shadow_map];

        let sliced_frustums = frustum_slice(original.camera.projection, Self::CASCADE_COUNT as u32);

        for (id, light) in &original.directional_lights {
            let cascade_views = sliced_frustums.clone().into_iter().map(|proj| {
                Self::calculate_cascade_view(original.camera.transform, proj, light.direction)
            });

            let mut cascade_maps = [TextureViewId::default(); Self::CASCADE_COUNT];

            for (i_cascade, cascade_view) in cascade_views.enumerate() {
                cascade_directional_desc.base_array_layer = directional_index;
                let texture_view_id = TextureViewId(Uuid::new_v4());
                cascade_maps[i_cascade] = texture_view_id;

                assets.texture_views.insert(
                    texture_view_id,
                    directional_shadow_maps.create_view(&cascade_directional_desc),
                );

                // bf_cascade_views.push(&cascade_view);
                raw_cascade_views.extend_from_slice(bytemuck::bytes_of(&cascade_view));
                self.offsets.push(bf_light_views.push(&cascade_view));
                directional_index += 1;
            }

            self.directional_views.insert(*id, cascade_maps);
        }

        for (id, light) in &original.point_lights {
            let light_views = light.light_view();
            let mut texture_views = [TextureViewId::default(); 6];

            for i_face in 0..6 {
                point_desc.base_array_layer = point_index * 6 + i_face as u32;
                let texture_view_id = TextureViewId(Uuid::new_v4());
                texture_views[i_face] = texture_view_id;
                assets
                    .texture_views
                    .insert(texture_view_id, point_shadow_maps.create_view(&point_desc));

                bf_point_light_view.push(&light_views[i_face].proj);
                self.offsets
                    .push(bf_light_views.push(&light_views[i_face].proj));
            }

            self.point_views.insert(*id, texture_views);
            point_index += 1;
        }

        for (id, light) in &original.spot_lights {
            let light_views = light.light_view();
            let mut texture_views = [TextureViewId::default(); 6];

            for i_face in 0..6 {
                point_desc.base_array_layer = point_index * 6 + i_face as u32;
                let texture_view_id = TextureViewId(Uuid::new_v4());
                texture_views[i_face] = texture_view_id;
                assets
                    .texture_views
                    .insert(texture_view_id, point_shadow_maps.create_view(&point_desc));

                bf_point_light_view.push(&light_views[i_face].proj);
                self.offsets
                    .push(bf_light_views.push(&light_views[i_face].proj));
            }

            self.point_views.insert(*id, texture_views);
            point_index += 1;
        }

        let mut bf_cascade_views = DynamicGpuBuffer::new(BufferUsages::STORAGE);
        bf_cascade_views.set(raw_cascade_views);

        bf_cascade_views.write::<GpuCamera>(&renderer.device, &renderer.queue);
        bf_point_light_view.write::<GpuCamera>(&renderer.device, &renderer.queue);
        bf_light_views.write::<GpuCamera>(&renderer.device, &renderer.queue);

        assets
            .extra_buffers
            .insert(SHADOW_MAPPING.cascade_views, bf_cascade_views);
        assets
            .extra_buffers
            .insert(SHADOW_MAPPING.point_light_views, bf_point_light_view);
        assets
            .extra_buffers
            .insert(SHADOW_MAPPING.light_views, bf_light_views);

        assets.extra_bind_groups.insert(
            SHADOW_MAPPING.light_views_bind_group,
            renderer.device.create_bind_group(&BindGroupDescriptor {
                label: Some("light_views_bind_group"),
                layout: &assets.extra_layouts[&SHADOW_MAPPING.light_view_layout],
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: assets.extra_buffers[&SHADOW_MAPPING.light_views]
                        .binding::<GpuCamera>()
                        .unwrap(),
                }],
            }),
        );

        assets.extra_bind_groups.insert(
            SHADOW_MAPPING.shadow_maps_bind_group,
            renderer.device.create_bind_group(&BindGroupDescriptor {
                label: Some("shadow_map_bind_group"),
                layout: &assets.extra_layouts[&SHADOW_MAPPING.shadow_maps_layout],
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: assets.extra_buffers[&SHADOW_MAPPING.cascade_views]
                            .entire_binding()
                            .unwrap(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: assets.extra_buffers[&SHADOW_MAPPING.point_light_views]
                            .entire_binding()
                            .unwrap(),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Sampler(
                            &assets.samplers[&SHADOW_MAPPING.shadow_map_sampler],
                        ),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::Sampler(
                            &assets.samplers[&SHADOW_MAPPING.shadow_texture_sampler],
                        ),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: BindingResource::TextureView(
                            &assets.texture_views[&SHADOW_MAPPING.directional_shadow_map_view],
                        ),
                    },
                    BindGroupEntry {
                        binding: 5,
                        resource: BindingResource::TextureView(
                            &assets.texture_views[&SHADOW_MAPPING.point_shadow_map_view],
                        ),
                    },
                    BindGroupEntry {
                        binding: 6,
                        resource: assets.extra_buffers[&SHADOW_MAPPING.poisson_disk]
                            .entire_binding()
                            .unwrap(),
                    },
                    BindGroupEntry {
                        binding: 7,
                        resource: assets.extra_buffers[&SHADOW_MAPPING.config]
                            .entire_binding()
                            .unwrap(),
                    },
                ],
            }),
        );
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
            .get(&SHADOW_MAPPING.light_views_bind_group)
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
            for texture_view_id in &self.directional_views[id] {
                _draw(&scene.assets.texture_views[&texture_view_id]);
            }
        }

        for id in scene.original.point_lights.keys() {
            for texture_view_id in &self.point_views[id] {
                _draw(&scene.assets.texture_views[&texture_view_id]);
            }
        }

        renderer.queue.submit([encoder.finish()]);
    }
}
