use std::cell::RefCell;

use aurora_core::{
    render::{
        flow::{NodeExtraData, PipelineCreationContext, RenderContext, RenderNode},
        resource::{DynamicGpuBuffer, Image},
        scene::{
            ExtraBindGroupId, ExtraBufferId, ExtraLayoutId, GpuScene, SamplerId, TextureId,
            TextureViewId,
        },
    },
    util::cube::CUBE_MAP_FACES,
};
use encase::ShaderType;
use glam::{Mat4, Vec3};
use image::ImageFormat;
use uuid::Uuid;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BufferBindingType, BufferUsages, Color, ColorTargetState,
    ColorWrites, ComputePipeline, ComputePipelineDescriptor, Extent3d, FragmentState, LoadOp,
    Operations, PipelineLayoutDescriptor, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, SamplerBindingType, SamplerDescriptor, ShaderStages,
    StorageTextureAccess, StoreOp, TextureDescriptor, TextureDimension, TextureFormat,
    TextureSampleType, TextureUsages, TextureViewDescriptor, TextureViewDimension, VertexState,
};

#[derive(ShaderType)]
pub struct CubeMapFace {
    pub view: Mat4,
    pub up: Vec3,
}

#[derive(ShaderType)]
pub struct EnvironmentMappingConfig {
    pub intensity: f32,
}

#[derive(ShaderType)]
pub struct EnvironmentMapConvolution {
    pub elevation_samples: u32,
    pub azimuth_samples: u32,
    pub sample_distance: f32,
}

pub struct EnvironmentMapping {
    pub refl_map_texture: TextureId,
    pub irradiant_map_texture: TextureId,
    pub env_map_sampler: SamplerId,

    pub refl_map_cube_view: TextureViewId,
    pub irradiant_map_cube_view: TextureViewId,

    pub env_mapping_layout: ExtraLayoutId,
    pub env_mapping_bind_group: ExtraBindGroupId,
    pub env_map_config: ExtraBufferId,
    pub sample_dirs: ExtraBufferId,

    pub env_mapping_convol_layout: ExtraLayoutId,
    pub env_mapping_convol_bind_group: ExtraBindGroupId,
    pub env_map_convol_config: ExtraBufferId,
}

pub const ENV_MAPPING: EnvironmentMapping = EnvironmentMapping {
    refl_map_texture: TextureId(Uuid::from_u128(78974561230215021548120154)),
    irradiant_map_texture: TextureId(Uuid::from_u128(1356431651062340305641035)),
    env_map_sampler: SamplerId(Uuid::from_u128(27313021528494090841905800393)),

    refl_map_cube_view: TextureViewId(Uuid::from_u128(841634093259944973159189)),
    irradiant_map_cube_view: TextureViewId(Uuid::from_u128(570610877763617449660266)),

    env_mapping_layout: ExtraLayoutId(Uuid::from_u128(12487544531485120554561230)),
    env_mapping_bind_group: ExtraBindGroupId(Uuid::from_u128(798465100154312025145463519945612)),
    env_map_config: ExtraBufferId(Uuid::from_u128(4856410345313210325401521354)),
    sample_dirs: ExtraBufferId(Uuid::from_u128(8945615648432452354345111)),

    env_mapping_convol_layout: ExtraLayoutId(Uuid::from_u128(458455557304384985223748528)),
    env_mapping_convol_bind_group: ExtraBindGroupId(Uuid::from_u128(263401621817287112631847950)),
    env_map_convol_config: ExtraBufferId(Uuid::from_u128(993340683210943589314581863)),
};

pub const ENVIRONMENT_MAP_PATH_ATTR: &'static str = "ENVIRONMENT_MAP";

#[derive(Default)]
pub struct EnvironmentMappingNode {
    pipeline: Option<RenderPipeline>,
    // view, sample_dir buffer offset
    irradiance_map_sliced_view: [(TextureViewId, u32); 6],
    cube_face_size: u32,
    generated: RefCell<bool>,
}

impl EnvironmentMappingNode {
    pub const CONFIG: EnvironmentMappingConfig = EnvironmentMappingConfig { intensity: 500. };
    pub const CONVOLUTION_CONFIG: EnvironmentMapConvolution = EnvironmentMapConvolution {
        elevation_samples: 2,
        azimuth_samples: 1,
        sample_distance: 0.,
    };
}

impl RenderNode for EnvironmentMappingNode {
    fn require_shader(&self) -> Option<(&'static [&'static str], &'static str)> {
        Some((
            &[
                include_str!("../shader/math.wgsl"),
                include_str!("../shader/fullscreen.wgsl"),
                include_str!("../shader/env_mapping/env_mapping_type.wgsl"),
            ],
            include_str!("../shader/env_mapping/convolve_env_map.wgsl"),
        ))
    }

    fn create_pipelines(
        &mut self,
        GpuScene { assets, .. }: &mut GpuScene,
        PipelineCreationContext { device, shader, .. }: PipelineCreationContext,
    ) {
        let convolution_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("environment_map_convolution_layout"),
            entries: &[
                // Src
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                },
                // Config
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(EnvironmentMapConvolution::min_size()),
                    },
                    count: None,
                },
                // Sample dir
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(CubeMapFace::min_size()),
                    },
                    count: None,
                },
            ],
        });

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("environment_map_convolution_pipeline_layout"),
            bind_group_layouts: &[&convolution_layout],
            ..Default::default()
        });

        self.pipeline = Some(device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("environment_map_convolution_pipeline"),
            layout: Some(&layout),
            vertex: VertexState {
                module: shader,
                entry_point: "vertex",
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: shader,
                entry_point: "fragment",
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba32Float,
                    blend: None,
                    write_mask: ColorWrites::all(),
                })],
            }),
            primitive: Default::default(),
            depth_stencil: Default::default(),
            multisample: Default::default(),
            multiview: Default::default(),
            cache: Default::default(),
        }));

        assets
            .extra_layouts
            .insert(ENV_MAPPING.env_mapping_convol_layout, convolution_layout);
    }

    fn build(
        &mut self,
        GpuScene { assets, .. }: &mut GpuScene,
        RenderContext {
            device,
            queue,
            node,
            ..
        }: RenderContext,
    ) {
        let Some(NodeExtraData::String(path)) = node.extra_data.get(ENVIRONMENT_MAP_PATH_ATTR)
        else {
            return;
        };

        let env_map_img =
            Image::from_buffer(&std::fs::read(path).unwrap(), ImageFormat::Hdr, false);
        // This method ensures that this is a valid cube map.
        // So width / 4 == height / 3
        let refl_map = env_map_img.as_cube_map(device, queue, &Default::default());
        self.cube_face_size = env_map_img.width() / 4;

        let refl_map_cube_view = refl_map.create_view(&TextureViewDescriptor {
            label: Some("reflection_map_cube_view"),
            dimension: Some(TextureViewDimension::Cube),
            ..Default::default()
        });

        let irradiance_map = device.create_texture(&TextureDescriptor {
            label: Some("irradiance_map"),
            size: Extent3d {
                width: self.cube_face_size,
                height: self.cube_face_size,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba32Float,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let irradiance_map_cube_view = irradiance_map.create_view(&TextureViewDescriptor {
            label: Some("irradiance_map_cube_view"),
            dimension: Some(TextureViewDimension::Cube),
            ..Default::default()
        });

        let mut bf_sample_faces = DynamicGpuBuffer::new(BufferUsages::UNIFORM);
        bf_sample_faces.set_label("sample_faces");

        for (index, face) in CUBE_MAP_FACES.into_iter().enumerate() {
            let view = irradiance_map.create_view(&TextureViewDescriptor {
                label: Some("irradiance_map_sliced_view"),
                dimension: Some(TextureViewDimension::D2),
                base_array_layer: index as u32,
                array_layer_count: Some(1),
                ..Default::default()
            });
            let id = TextureViewId(Uuid::new_v4());
            let offset = bf_sample_faces.push(&CubeMapFace {
                view: Mat4::look_to_rh(Vec3::ZERO, face.target, face.up).inverse(),
                up: face.up,
            });

            self.irradiance_map_sliced_view[index] = (id, offset);
            assets.texture_views.insert(id, view);
        }

        bf_sample_faces.write::<CubeMapFace>(device, queue);

        let env_map_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("environment_map_sampler"),
            ..Default::default()
        });

        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("environment_mapping_layout"),
            entries: &[
                // Unfiltered Reflection Map
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                // Filtered Irradiant Map
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                },
                // Config
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(EnvironmentMappingConfig::min_size()),
                    },
                    count: None,
                },
            ],
        });

        let mut bf_config = DynamicGpuBuffer::new(BufferUsages::UNIFORM);
        bf_config.set_label("environment_map_convolution_config");
        bf_config.push(&Self::CONFIG);
        bf_config.write::<EnvironmentMappingConfig>(device, queue);

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("environment_mapping_bind_group"),
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&refl_map_cube_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&irradiance_map_cube_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&env_map_sampler),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: bf_config.entire_binding().unwrap(),
                },
            ],
        });

        let mut bf_convol_config = DynamicGpuBuffer::new(BufferUsages::UNIFORM);
        bf_convol_config.push(&EnvironmentMapConvolution {
            sample_distance: self.cube_face_size as f32,
            ..Self::CONVOLUTION_CONFIG
        });
        bf_convol_config.write::<EnvironmentMapConvolution>(device, queue);

        let convolution_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("environment_map_convolution_bind_group"),
            layout: &assets.extra_layouts[&ENV_MAPPING.env_mapping_convol_layout],
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&refl_map_cube_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&env_map_sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: bf_convol_config.entire_binding().unwrap(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: bf_sample_faces.binding::<CubeMapFace>().unwrap(),
                },
            ],
        });

        assets
            .extra_layouts
            .insert(ENV_MAPPING.env_mapping_layout, layout);
        assets
            .extra_bind_groups
            .insert(ENV_MAPPING.env_mapping_bind_group, bind_group);
        assets.extra_bind_groups.insert(
            ENV_MAPPING.env_mapping_convol_bind_group,
            convolution_bind_group,
        );
        assets
            .textures
            .insert(ENV_MAPPING.refl_map_texture, refl_map);
        assets
            .textures
            .insert(ENV_MAPPING.irradiant_map_texture, irradiance_map);
        assets
            .texture_views
            .insert(ENV_MAPPING.refl_map_cube_view, refl_map_cube_view);
        assets.texture_views.insert(
            ENV_MAPPING.irradiant_map_cube_view,
            irradiance_map_cube_view,
        );
        assets
            .samplers
            .insert(ENV_MAPPING.env_map_sampler, env_map_sampler);
        assets
            .extra_buffers
            .insert(ENV_MAPPING.env_map_config, bf_config);
        assets
            .extra_buffers
            .insert(ENV_MAPPING.env_map_convol_config, bf_convol_config);
        assets
            .extra_buffers
            .insert(ENV_MAPPING.sample_dirs, bf_sample_faces);
    }

    fn draw(
        &self,
        GpuScene { assets, .. }: &mut GpuScene,
        RenderContext { device, queue, .. }: RenderContext,
    ) {
        if *self.generated.borrow() {
            return;
        }
        self.generated.replace(true);

        let mut command_encoder = device.create_command_encoder(&Default::default());
        for (target, offset) in self.irradiance_map_sliced_view {
            let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("environment_map_convolution_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &assets.texture_views[&target],
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            pass.set_pipeline(self.pipeline.as_ref().unwrap());
            pass.set_bind_group(
                0,
                &assets.extra_bind_groups[&ENV_MAPPING.env_mapping_convol_bind_group],
                &[offset],
            );
            pass.draw(0..3, 0..1);
        }

        queue.submit([command_encoder.finish()]);
    }
}
