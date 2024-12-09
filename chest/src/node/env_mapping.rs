use std::path::PathBuf;

use aurora_core::{
    render::{
        flow::{RenderContext, RenderNode},
        resource::{DynamicGpuBuffer, Image},
        scene::{ExtraBindGroupId, ExtraBufferId, ExtraLayoutId, GpuScene, SamplerId, TextureId},
    },
    util::cube::CUBE_MAP_FACES,
};
use encase::ShaderType;
use glam::{Mat4, Vec3};
use image::ImageFormat;
use uuid::Uuid;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BufferBindingType, BufferUsages,
    ColorTargetState, ColorWrites, Extent3d, Features, FilterMode, FragmentState,
    PipelineLayoutDescriptor, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, SamplerBindingType, SamplerDescriptor, ShaderStages,
    TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
    TextureView, TextureViewDescriptor, TextureViewDimension, VertexState,
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

impl Default for EnvironmentMappingConfig {
    fn default() -> Self {
        Self { intensity: 1000. }
    }
}

pub struct EnvironmentMappingNodeConfig {
    pub env_map_path: PathBuf,
}

#[derive(ShaderType)]
pub struct EnvironmentMapConvolutionConfig {
    pub elevation_samples: u32,
    pub azimuth_samples: u32,
    pub sample_distance: f32,
}

impl Default for EnvironmentMapConvolutionConfig {
    fn default() -> Self {
        Self {
            elevation_samples: 8,
            azimuth_samples: 8,
            sample_distance: 0.,
        }
    }
}

pub struct EnvironmentMappingData {
    pub irradiance_faces: Vec<(TextureView, u32)>,
    pub convolution_pipeline: RenderPipeline,
    pub convolution_bind_group: BindGroup,
}

pub struct EnvironmentMapping {
    pub env_map_texture: TextureId,
    pub env_map_sampler: SamplerId,

    pub env_mapping_layout: ExtraLayoutId,
    pub env_mapping_bind_group: ExtraBindGroupId,
    pub env_map_config: ExtraBufferId,
}

pub const ENV_MAPPING: EnvironmentMapping = EnvironmentMapping {
    env_map_texture: TextureId(Uuid::from_u128(78974561230215021548120154)),
    env_map_sampler: SamplerId(Uuid::from_u128(27313021528494090841905800393)),
    env_mapping_layout: ExtraLayoutId(Uuid::from_u128(12487544531485120554561230)),
    env_mapping_bind_group: ExtraBindGroupId(Uuid::from_u128(798465100154312025145463519945612)),
    env_map_config: ExtraBufferId(Uuid::from_u128(4856410345313210325401521354)),
};

pub const ENVIRONMENT_MAP_PATH_ATTR: &'static str = "ENVIRONMENT_MAP";

pub struct EnvironmentMappingNode {
    pub node_config: EnvironmentMappingNodeConfig,
    pub config: EnvironmentMappingConfig,
    pub convolution_config: EnvironmentMapConvolutionConfig,

    pub data: Option<EnvironmentMappingData>,
}

impl RenderNode for EnvironmentMappingNode {
    fn require_renderer_features(&self, features: &mut Features) {
        *features = Features::FLOAT32_FILTERABLE
    }

    fn require_shaders(&self) -> Option<&'static [(&'static [&'static str], &'static str)]> {
        Some(&[
            (&[], include_str!("../shader/fullscreen.wgsl")),
            (
                &[
                    include_str!("../shader/math.wgsl"),
                    include_str!("../shader/fullscreen.wgsl"),
                ],
                include_str!("../shader/env_mapping/convolve_env_map.wgsl"),
            ),
        ])
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
        let specular_texture = Image::from_buffer(
            &std::fs::read(&self.node_config.env_map_path).unwrap(),
            ImageFormat::Hdr,
            false,
        )
        .to_cube_map(device, queue, &Default::default());
        let cube_face_size = specular_texture.width();

        let irradiance_texture = device.create_texture(&TextureDescriptor {
            label: Some("irradiance_texture"),
            size: Extent3d {
                width: cube_face_size,
                height: cube_face_size,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba32Float,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let irradiance_texture_view = irradiance_texture.create_view(&TextureViewDescriptor {
            label: Some("irradiance_texture_view"),
            dimension: Some(TextureViewDimension::Cube),
            ..Default::default()
        });

        let specular_texture_view = specular_texture.create_view(&TextureViewDescriptor {
            label: Some("specular_texture_texture_view"),
            dimension: Some(TextureViewDimension::Cube),
            ..Default::default()
        });

        let env_map_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("environment_map_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        let env_mapping_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("env_map_convolution_layout"),
            entries: &[
                // Unfiltered Environment Map
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                // Irradiance Map
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
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
        bf_config.push(&self.config);
        bf_config.write::<EnvironmentMappingConfig>(device, queue);

        let env_mapping_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("env_mapping_bind_group"),
            layout: &env_mapping_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&specular_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&irradiance_texture_view),
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

        assets
            .extra_layouts
            .insert(ENV_MAPPING.env_mapping_layout, env_mapping_layout);
        assets
            .extra_bind_groups
            .insert(ENV_MAPPING.env_mapping_bind_group, env_mapping_bind_group);

        let mut bf_convolution_config = DynamicGpuBuffer::new(BufferUsages::UNIFORM);
        bf_convolution_config.push(&EnvironmentMapConvolutionConfig {
            sample_distance: cube_face_size as f32,
            ..self.convolution_config
        });
        bf_convolution_config.write::<EnvironmentMappingConfig>(device, queue);

        let mut bf_sample_faces = DynamicGpuBuffer::new(BufferUsages::UNIFORM);
        let mut irradiance_faces = Vec::with_capacity(6);

        for (index, face) in CUBE_MAP_FACES.into_iter().enumerate() {
            let view = irradiance_texture.create_view(&TextureViewDescriptor {
                label: Some("irradiance_texture_sliced_view"),
                dimension: Some(TextureViewDimension::D2),
                base_array_layer: index as u32,
                array_layer_count: Some(1),
                ..Default::default()
            });
            let offset = bf_sample_faces.push(&CubeMapFace {
                view: Mat4::look_to_rh(Vec3::ZERO, face.target, face.up).inverse(),
                up: face.up,
            });

            irradiance_faces.push((view, offset));
        }

        bf_sample_faces.write::<CubeMapFace>(device, queue);

        let convolution_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("specular_texture_convolution_layout"),
            entries: &[
                // Src
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                // Config
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(EnvironmentMapConvolutionConfig::min_size()),
                    },
                    count: None,
                },
                // Sample face
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

        let convolution_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("specular_texture_convolution_bind_group"),
            layout: &convolution_layout,
            entries: &[
                // Src
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&specular_texture_view),
                },
                // Sampler
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&env_map_sampler),
                },
                // Config
                BindGroupEntry {
                    binding: 2,
                    resource: bf_convolution_config.entire_binding().unwrap(),
                },
                // Face
                BindGroupEntry {
                    binding: 3,
                    resource: bf_sample_faces.binding::<CubeMapFace>().unwrap(),
                },
            ],
        });

        let convolution_pipeline_layout =
            device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("specular_texture_convolution_pipeline_layout"),
                bind_group_layouts: &[&convolution_layout],
                ..Default::default()
            });

        let convolution_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("specular_texture_convolution_pipeline"),
            layout: Some(&convolution_pipeline_layout),
            vertex: VertexState {
                module: &node.shaders[0],
                entry_point: "vertex",
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &node.shaders[1],
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
        });

        self.data = Some(EnvironmentMappingData {
            irradiance_faces,
            convolution_bind_group,
            convolution_pipeline,
        });
    }

    fn draw(&self, _scene: &mut GpuScene, RenderContext { device, queue, .. }: RenderContext) {
        let Some(EnvironmentMappingData {
            irradiance_faces,
            convolution_pipeline,
            convolution_bind_group,
        }) = &self.data
        else {
            return;
        };

        let mut command_encoder = device.create_command_encoder(&Default::default());

        for (target, offset) in irradiance_faces {
            let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("environment_map_convolution_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: Default::default(),
                })],
                ..Default::default()
            });

            pass.set_pipeline(convolution_pipeline);
            pass.set_bind_group(0, convolution_bind_group, &[*offset]);
            pass.draw(0..3, 0..1);
        }

        queue.submit([command_encoder.finish()]);
    }
}
