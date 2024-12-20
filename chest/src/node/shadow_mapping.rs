use std::collections::HashMap;

use aurora_core::render::{
    flow::{RenderContext, RenderNode},
    helper::{CameraProjection, Transform},
    resource::{DynamicGpuBuffer, GpuCamera},
    scene::{
        ExtraBindGroupId, ExtraBufferId, ExtraLayoutId, GpuScene, SamplerId, TextureId,
        TextureViewId,
    },
    ShaderDefEnum,
};
use encase::ShaderType;
use glam::{Mat4, Vec2, Vec3, Vec4};
use naga_oil::compose::ShaderDefValue;
use uuid::Uuid;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BufferBindingType, BufferUsages, CompareFunction, DepthBiasState,
    DepthStencilState, Extent3d, Face, Features, FilterMode, FragmentState, LoadOp,
    MultisampleState, Operations, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipelineDescriptor, SamplerBindingType, SamplerDescriptor, ShaderStages, StencilState,
    StoreOp, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, VertexBufferLayout,
    VertexFormat, VertexState, VertexStepMode,
};

use crate::{
    shader_defs::ShadowFiltering,
    util::{self, frustum_slice, Aabb},
};

bitflags::bitflags! {
    #[derive(Default)]
    pub struct ShadowMappingNodeConfig : u32 {
        const RANDOMIZE = 1 << 0;
    }
}

pub enum ShadowMapPartitioning {
    PSSM(u32),
    SDSM(u32),
}

#[derive(Default)]
pub enum DepthBiasing {
    NormalOffset,
    #[default]
    SingleSideRendering,
}

#[derive(ShaderType)]
pub struct ShadowMappingConfig {
    pub dir_map_resolution: u32,
    pub point_map_resolution: u32,
    pub samples: u32,
    pub dir_pcf_radius: f32,
    pub dir_pcss_radius: f32,
    pub point_pcf_radius: f32,
    pub point_pcss_radius: f32,
}

impl Default for ShadowMappingConfig {
    fn default() -> Self {
        Self {
            dir_map_resolution: 2048,
            point_map_resolution: 512,
            samples: 16,
            dir_pcf_radius: 1.,
            dir_pcss_radius: 1.,
            point_pcf_radius: 0.2,
            point_pcss_radius: 0.1,
        }
    }
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

pub struct ShadowMappingNode {
    pub config: ShadowMappingConfig,
    pub partitioning: Option<ShadowMapPartitioning>,
    pub filtering: Option<ShadowFiltering>,
    pub depth_biasing: DepthBiasing,
    pub show_cascades: bool,
    pub node_cfg: ShadowMappingNodeConfig,

    pub directional_views: HashMap<Uuid, Vec<TextureViewId>>,
    pub point_views: HashMap<Uuid, [TextureViewId; 6]>,
    pub offsets: Vec<u32>,
}

impl Default for ShadowMappingNode {
    fn default() -> Self {
        Self {
            config: Default::default(),
            partitioning: Some(ShadowMapPartitioning::PSSM(3)),
            filtering: Some(ShadowFiltering::PCSS),
            depth_biasing: Default::default(),
            node_cfg: Default::default(),
            show_cascades: Default::default(),
            directional_views: Default::default(),
            point_views: Default::default(),
            offsets: Default::default(),
        }
    }
}

impl ShadowMappingNode {
    pub fn cascade_count(&self) -> u32 {
        match &self.partitioning {
            Some(p) => match p {
                ShadowMapPartitioning::PSSM(n) => *n,
                ShadowMapPartitioning::SDSM(n) => *n,
            },
            None => 1,
        }
    }

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
        let half_aabb_size = (cascade_proj_aabb.max - cascade_proj_aabb.min) * 0.6;

        let cascade_proj = Mat4::orthographic_rh(
            -half_aabb_size.x,
            half_aabb_size.x,
            -half_aabb_size.y,
            half_aabb_size.y,
            -half_aabb_size.z,
            half_aabb_size.z,
        );

        GpuCamera {
            view: cascade_view,
            inv_view: cascade_view.inverse(),
            proj: cascade_proj,
            inv_proj: cascade_proj.inverse(),
            position_ws: match camera_proj_slice {
                CameraProjection::Perspective(_) => center,
                CameraProjection::Orthographic(_) => light_dir,
            },
            // SPECIAL USE CASE!!
            exposure: match camera_proj_slice {
                CameraProjection::Perspective(proj) => proj.near,
                CameraProjection::Orthographic(proj) => proj.near,
            },
        }
    }
}

impl RenderNode for ShadowMappingNode {
    fn restrict_mesh_format(&self) -> Option<&'static [VertexFormat]> {
        Some(&[
            VertexFormat::Float32x3,
            VertexFormat::Float32x3,
            VertexFormat::Float32x2,
            VertexFormat::Float32x4,
        ])
    }

    fn require_renderer_features(&self, features: &mut Features) {
        *features |= Features::DEPTH_CLIP_CONTROL;
    }

    fn require_shader_defs(&self, shader_defs: &mut HashMap<String, ShaderDefValue>) {
        shader_defs.extend([(
            "SHADOW_CASCADES".to_owned(),
            ShaderDefValue::UInt(self.cascade_count()),
        )]);

        if self.node_cfg.contains(ShadowMappingNodeConfig::RANDOMIZE) {
            shader_defs.insert(
                "SHADOW_MAP_SAMPLE_RANDOMIZE".to_string(),
                ShaderDefValue::Bool(true),
            );
        }

        if let Some(filtering) = &self.filtering {
            shader_defs.extend([filtering.to_def()]);
        }

        match self.depth_biasing {
            DepthBiasing::NormalOffset => {
                shader_defs.insert("NORMAL_OFFSET".to_string(), ShaderDefValue::Bool(true));
            }
            _ => {}
        }
    }

    fn require_shaders(&self) -> Option<&'static [(&'static [&'static str], &'static str)]> {
        Some(&[(
            &[
                include_str!("../shader/math.wgsl"),
                include_str!("../shader/common/common_type.wgsl"),
                include_str!("../shader/common/common_binding.wgsl"),
                include_str!("../shader/shadow/shadow_type.wgsl"),
            ],
            include_str!("../shader/shadow/shadow_render.wgsl"),
        )])
    }

    fn build(
        &mut self,
        GpuScene {
            original, assets, ..
        }: &mut GpuScene,
        RenderContext {
            device,
            queue,
            node,
            targets,
        }: RenderContext,
    ) {
        let directional_shadow_map = device.create_texture(&TextureDescriptor {
            label: Some("directional_shadow_map"),
            size: Extent3d {
                width: self.config.dir_map_resolution,
                height: self.config.dir_map_resolution,
                depth_or_array_layers: self.cascade_count().max(1),
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

        let point_shadow_map = device.create_texture(&TextureDescriptor {
            label: Some("point_shadow_map"),
            size: Extent3d {
                width: self.config.point_map_resolution,
                height: self.config.point_map_resolution,
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

        let shadow_map_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("shadow_map_sampler"),
            compare: Some(CompareFunction::LessEqual),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        let shadow_texture_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("shadow_texture_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        let shadow_maps_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("shadow_maps_layout"),
            entries: &[
                // Directional/Cascade Views
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: Some(<GpuCamera as encase::ShaderType>::min_size()),
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
                        min_binding_size: Some(<GpuCamera as encase::ShaderType>::min_size()),
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
                        min_binding_size: Some(<Vec4 as encase::ShaderType>::min_size()),
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
            directional_shadow_map,
        );
        assets.texture_views.insert(
            SHADOW_MAPPING.directional_shadow_map_view,
            directional_shadow_map_view,
        );
        assets
            .textures
            .insert(SHADOW_MAPPING.point_shadow_map, point_shadow_map);
        assets
            .texture_views
            .insert(SHADOW_MAPPING.point_shadow_map_view, point_shadow_map_view);

        let mut bf_poisson_disk = DynamicGpuBuffer::new(BufferUsages::STORAGE);
        let mut raw_poisson_disk = Vec::new();
        fast_poisson::Poisson2D::new()
            .into_iter()
            .take(self.config.samples as usize)
            .for_each(|x| {
                raw_poisson_disk.extend_from_slice(bytemuck::bytes_of(
                    &(Vec2::from_array(x) * 2. - 1.).extend(0.).extend(0.),
                ));
            });

        fast_poisson::Poisson3D::new()
            .into_iter()
            .take(self.config.samples as usize)
            .for_each(|x| {
                let p = Vec3::from_array(x) * 2. - 1.;
                raw_poisson_disk.extend_from_slice(bytemuck::bytes_of(&p.extend(0.)));
            });

        bf_poisson_disk.set(raw_poisson_disk);
        bf_poisson_disk.write::<Vec4>(&device, &queue);
        assets
            .extra_buffers
            .insert(SHADOW_MAPPING.poisson_disk, bf_poisson_disk);

        let mut bf_config = DynamicGpuBuffer::new(BufferUsages::UNIFORM);
        bf_config.push(&self.config);
        bf_config.write::<ShadowMappingConfig>(&device, &queue);
        assets
            .extra_buffers
            .insert(SHADOW_MAPPING.config, bf_config);

        let light_view_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(<GpuCamera as encase::ShaderType>::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
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

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("shadow_mapping_shader"),
            bind_group_layouts: &[&light_view_layout],
            push_constant_ranges: &[],
        });

        assets
            .extra_layouts
            .insert(SHADOW_MAPPING.light_view_layout, light_view_layout);

        for mesh in &node.meshes {
            if node.pipelines.contains_key(&mesh.mesh.mesh) {
                continue;
            };

            let instance = &assets.meshes[&mesh.mesh.mesh];
            let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("shadow_mapping_pipeline"),
                layout: Some(&layout),
                cache: None,
                vertex: VertexState {
                    module: &node.shaders[0],
                    entry_point: "vertex",
                    compilation_options: PipelineCompilationOptions::default(),
                    buffers: &[VertexBufferLayout {
                        array_stride: instance.vertex_stride(),
                        step_mode: VertexStepMode::Vertex,
                        attributes: &instance.vertex_attributes(),
                    }],
                },
                fragment: Some(FragmentState {
                    module: &node.shaders[0],
                    entry_point: "fragment",
                    compilation_options: PipelineCompilationOptions::default(),
                    targets: &[None],
                }),
                multisample: MultisampleState::default(),
                depth_stencil: Some(DepthStencilState {
                    format: targets.depth_format.unwrap(),
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::LessEqual,
                    stencil: StencilState::default(),
                    bias: DepthBiasState::default(),
                }),
                primitive: PrimitiveState {
                    cull_mode: match self.depth_biasing {
                        DepthBiasing::NormalOffset => None,
                        DepthBiasing::SingleSideRendering => Some(Face::Front),
                    },
                    unclipped_depth: true,
                    ..Default::default()
                },
                multiview: None,
            });
            node.pipelines.insert(mesh.mesh.mesh, pipeline);
        }
    }

    fn prepare(
        &mut self,
        GpuScene {
            original, assets, ..
        }: &mut GpuScene,
        RenderContext { device, queue, .. }: RenderContext,
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

        let mut raw_cascade_views = Vec::new();
        let mut bf_point_light_view = DynamicGpuBuffer::new(BufferUsages::STORAGE);
        let mut bf_light_views = DynamicGpuBuffer::new(BufferUsages::UNIFORM);

        let directional_shadow_maps = &assets.textures[&SHADOW_MAPPING.directional_shadow_map];
        let point_shadow_maps = &assets.textures[&SHADOW_MAPPING.point_shadow_map];

        let sliced_frustums = frustum_slice(original.camera.projection, self.cascade_count(), 0.5);

        for (id, light) in &original.dir_lights {
            let cascade_views = sliced_frustums.clone().into_iter().map(|proj| {
                Self::calculate_cascade_view(original.camera.transform, proj, light.direction)
            });

            let mut cascade_maps = Vec::new();

            for cascade_view in cascade_views {
                directional_desc.base_array_layer = directional_index;
                let texture_view_id = TextureViewId(Uuid::new_v4());
                cascade_maps.push(texture_view_id);

                assets.texture_views.insert(
                    texture_view_id,
                    directional_shadow_maps.create_view(&directional_desc),
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

                bf_point_light_view.push(&light_views[i_face]);
                self.offsets.push(bf_light_views.push(&light_views[i_face]));
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

                bf_point_light_view.push(&light_views[i_face]);
                self.offsets.push(bf_light_views.push(&light_views[i_face]));
            }

            self.point_views.insert(*id, texture_views);
            point_index += 1;
        }

        let mut bf_cascade_views = DynamicGpuBuffer::new(BufferUsages::STORAGE);
        bf_cascade_views.set(raw_cascade_views);

        bf_cascade_views.write::<GpuCamera>(&device, &queue);
        bf_point_light_view.write::<GpuCamera>(&device, &queue);
        bf_light_views.write::<GpuCamera>(&device, &queue);

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
            device.create_bind_group(&BindGroupDescriptor {
                label: Some("light_views_bind_group"),
                layout: &assets.extra_layouts[&SHADOW_MAPPING.light_view_layout],
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: assets.extra_buffers[&SHADOW_MAPPING.light_views]
                            .binding::<GpuCamera>()
                            .unwrap(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: assets.extra_buffers[&SHADOW_MAPPING.config]
                            .entire_binding()
                            .unwrap(),
                    },
                ],
            }),
        );

        assets.extra_bind_groups.insert(
            SHADOW_MAPPING.shadow_maps_bind_group,
            device.create_bind_group(&BindGroupDescriptor {
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
        GpuScene {
            original, assets, ..
        }: &mut GpuScene,
        RenderContext {
            device,
            queue,
            node,
            ..
        }: RenderContext,
    ) {
        let Some(light_view_bind_groups) = assets
            .extra_bind_groups
            .get(&SHADOW_MAPPING.light_views_bind_group)
        else {
            return;
        };

        let mut view_index = 0;
        let mut encoder = device.create_command_encoder(&Default::default());

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

            pass.set_bind_group(0, light_view_bind_groups, &[self.offsets[view_index]]);

            for mesh in &node.meshes {
                let (Some(pipeline), Some(instance)) = (
                    node.pipelines.get(&mesh.mesh.mesh),
                    assets.gpu_meshes.get(&mesh.mesh.mesh),
                ) else {
                    continue;
                };

                pass.set_pipeline(pipeline);
                pass.set_vertex_buffer(0, instance.vertex_buffer.slice(..));
                if let Some(indices) = &instance.index_buffer {
                    pass.set_index_buffer(indices.buffer.slice(..), indices.format);
                    pass.draw_indexed(0..indices.count, 0, 0..1);
                } else {
                    pass.draw(0..instance.vertices_count, 0..1);
                }
            }

            view_index += 1;
        };

        for id in original.dir_lights.keys() {
            for texture_view_id in &self.directional_views[id] {
                _draw(&assets.texture_views[&texture_view_id]);
            }
        }

        for id in original
            .point_lights
            .keys()
            .chain(original.spot_lights.keys())
        {
            for texture_view_id in &self.point_views[id] {
                _draw(&assets.texture_views[&texture_view_id]);
            }
        }

        queue.submit([encoder.finish()]);
    }
}
