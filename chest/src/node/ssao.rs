use std::collections::HashMap;

use aurora_core::render::{
    flow::{RenderContext, RenderNode},
    resource::{DynamicGpuBuffer, GpuCamera},
    scene::{
        ExtraBindGroupId, ExtraBufferId, ExtraLayoutId, GpuScene, SamplerId, TextureId,
        TextureViewId,
    },
};
use encase::ShaderType;
use glam::UVec2;
use naga_oil::compose::ShaderDefValue;
use uuid::Uuid;
use wgpu::{
    util::DeviceExt, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BufferBindingType, BufferUsages,
    ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor, Device, Extent3d,
    PipelineLayoutDescriptor, Queue, SamplerBindingType, SamplerDescriptor, ShaderStages,
    StorageTextureAccess, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureSampleType, TextureUsages, TextureViewDescriptor, TextureViewDimension,
};

use crate::node::{DEPTH_PREPASS_TEXTURE, NORMAL_PREPASS_TEXTURE};

#[derive(ShaderType)]
pub struct SsaoConfig {
    pub texture_dim: UVec2,
    pub slices: u32,
    pub samples: u32,
    pub strength: f32,
    pub angle_bias: f32,
    pub max_depth_diff: f32,
}

pub struct Ssao {
    pub noisy_ssao_texture: TextureId,
    pub noisy_ssao_texture_view: TextureViewId,
    pub ssao_texture: TextureId,
    pub ssao_texture_view: TextureViewId,

    pub hilbert_lut: TextureId,
    pub hilbert_lut_view: TextureViewId,
    pub ssao_compute_layout: ExtraLayoutId,
    pub ssao_compute_bind_group: ExtraBindGroupId,
    pub ssao_denoise_layout: ExtraLayoutId,
    pub ssao_denoise_bind_group: ExtraBindGroupId,
    pub ssao_config: ExtraBufferId,

    pub ssao_layout: ExtraLayoutId,
    pub ssao_bind_group: ExtraBindGroupId,
    pub ssao_sampler: SamplerId,
}

impl Default for SsaoConfig {
    fn default() -> Self {
        Self {
            texture_dim: UVec2::ZERO,
            slices: 4,
            samples: 4,
            strength: 16.0,
            angle_bias: std::f32::consts::FRAC_PI_3,
            // angle_bias: 0.0,
            max_depth_diff: 2.0,
        }
    }
}

pub const SSAO: Ssao = Ssao {
    noisy_ssao_texture: TextureId(Uuid::from_u128(4365164098645125120)),
    noisy_ssao_texture_view: TextureViewId(Uuid::from_u128(15633484787465123021548)),
    ssao_texture: TextureId(Uuid::from_u128(41015468965230156489512014586201)),
    ssao_texture_view: TextureViewId(Uuid::from_u128(4510145121465015321456848476513)),
    hilbert_lut: TextureId(Uuid::from_u128(84323135153291223107)),
    hilbert_lut_view: TextureViewId(Uuid::from_u128(69300495220279111993)),

    ssao_compute_layout: ExtraLayoutId(Uuid::from_u128(153014631045364165)),
    ssao_compute_bind_group: ExtraBindGroupId(Uuid::from_u128(843650453641368049)),
    ssao_denoise_layout: ExtraLayoutId(Uuid::from_u128(8674601848654309867435)),
    ssao_denoise_bind_group: ExtraBindGroupId(Uuid::from_u128(96457666268395106319)),
    ssao_config: ExtraBufferId(Uuid::from_u128(646846350026484867986486997)),

    ssao_layout: ExtraLayoutId(Uuid::from_u128(436840684109684365013)),
    ssao_bind_group: ExtraBindGroupId(Uuid::from_u128(4887674863516530486789645)),
    ssao_sampler: SamplerId(Uuid::from_u128(1464060146365201068451)),
};

pub const SSAO_TEXTURE_FORMAT: TextureFormat = TextureFormat::R32Float;

#[derive(Default)]
pub struct SsaoNode {
    pub config: SsaoConfig,
    pub denoise: bool,

    pub compute_pipeline: Option<ComputePipeline>,
    pub denoise_pipeline: Option<ComputePipeline>,
}

const HILBERT_WIDTH: u16 = 64;

impl SsaoNode {
    pub const SSAO_WORKGROUP_SIZE: u32 = 16;

    pub fn generate_hilbert_lut(device: &Device, queue: &Queue) -> Texture {
        let mut t = [[0; 64]; 64];
        for x in 0..64 {
            for y in 0..64 {
                t[x][y] = Self::hilbert_index(x as u16, y as u16);
            }
        }

        device.create_texture_with_data(
            queue,
            &TextureDescriptor {
                label: Some("hilbert_lut"),
                size: Extent3d {
                    width: HILBERT_WIDTH as u32,
                    height: HILBERT_WIDTH as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R16Uint,
                usage: TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            Default::default(),
            bytemuck::cast_slice(&t),
        )
    }

    // Bevy
    // https://www.shadertoy.com/view/3tB3z3
    fn hilbert_index(mut x: u16, mut y: u16) -> u16 {
        let mut index = 0;

        let mut level: u16 = HILBERT_WIDTH / 2;
        while level > 0 {
            let region_x = (x & level > 0) as u16;
            let region_y = (y & level > 0) as u16;
            index += level * level * ((3 * region_x) ^ region_y);

            if region_y == 0 {
                if region_x == 1 {
                    x = HILBERT_WIDTH - 1 - x;
                    y = HILBERT_WIDTH - 1 - y;
                }

                std::mem::swap(&mut x, &mut y);
            }

            level /= 2;
        }

        index
    }
}

impl RenderNode for SsaoNode {
    fn require_shader_defs(&self, shader_defs: &mut HashMap<String, ShaderDefValue>) {
        shader_defs.insert(
            "SSAO_WORKGROUP_SIZE".to_string(),
            ShaderDefValue::UInt(Self::SSAO_WORKGROUP_SIZE),
        );
    }

    fn require_shaders(&self) -> Option<&'static [(&'static [&'static str], &'static str)]> {
        Some(&[
            (
                &[
                    include_str!("../shader/common/common_type.wgsl"),
                    include_str!("../shader/math.wgsl"),
                    include_str!("../shader/hash.wgsl"),
                ],
                include_str!("../shader/post_processing/ssao_compute.wgsl"),
            ),
            (
                &[include_str!("../shader/math.wgsl")],
                include_str!("../shader/post_processing/ssao_denoise.wgsl"),
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
            targets,
            ..
        }: RenderContext,
    ) {
        let compute_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("ssao_compute_layout"),
            entries: &[
                // Depth
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Normal
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Output AO
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::WriteOnly,
                        format: SSAO_TEXTURE_FORMAT,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Config
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(SsaoConfig::min_size()),
                    },
                    count: None,
                },
                // Sampler
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                },
                // Camera
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(GpuCamera::min_size()),
                    },
                    count: None,
                },
                // Noise
                BindGroupLayoutEntry {
                    binding: 6,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("ssao_pipeline_layout"),
            bind_group_layouts: &[&compute_layout],
            push_constant_ranges: &[],
        });

        self.compute_pipeline = Some(device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("ssao_pipeline"),
            layout: Some(&pipeline_layout),
            module: &node.shaders[0],
            entry_point: "main",
            compilation_options: Default::default(),
            cache: None,
        }));

        let noisy_ssao_texture = device.create_texture(&TextureDescriptor {
            label: Some("noisy_ssao_texture"),
            size: Extent3d {
                width: targets.size.x,
                height: targets.size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: SSAO_TEXTURE_FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let noisy_ssao_texture_view = noisy_ssao_texture.create_view(&TextureViewDescriptor {
            label: Some("noisy_ssao_texture"),
            ..Default::default()
        });

        let ssao_texture = device.create_texture(&TextureDescriptor {
            label: Some("ssao_texture"),
            size: Extent3d {
                width: targets.size.x,
                height: targets.size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: SSAO_TEXTURE_FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let ssao_texture_view = ssao_texture.create_view(&TextureViewDescriptor {
            label: Some("ssao_texture_view"),
            ..Default::default()
        });

        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("ssao_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("ssao_sampler"),
            ..Default::default()
        });

        assets
            .extra_layouts
            .insert(SSAO.ssao_compute_layout, compute_layout);
        assets.extra_layouts.insert(SSAO.ssao_layout, layout);
        assets
            .textures
            .insert(SSAO.noisy_ssao_texture, noisy_ssao_texture);
        assets
            .texture_views
            .insert(SSAO.noisy_ssao_texture_view, noisy_ssao_texture_view);
        assets.textures.insert(SSAO.ssao_texture, ssao_texture);
        assets
            .texture_views
            .insert(SSAO.ssao_texture_view, ssao_texture_view);
        assets.samplers.insert(SSAO.ssao_sampler, sampler);

        let denoise_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("ssao_denoise_layout"),
            entries: &[
                // Config
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(SsaoConfig::min_size()),
                    },
                    count: None,
                },
                // Src
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                },
                // Dst
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::WriteOnly,
                        format: SSAO_TEXTURE_FORMAT,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("ssao_denoise_pipeline_layout"),
            bind_group_layouts: &[&denoise_layout],
            ..Default::default()
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("ssao_denoise_pipeline"),
            layout: Some(&pipeline_layout),
            module: &node.shaders[1],
            entry_point: "main",
            compilation_options: Default::default(),
            cache: None,
        });

        self.denoise_pipeline = Some(pipeline);
        assets
            .extra_layouts
            .insert(SSAO.ssao_denoise_layout, denoise_layout);

        let hilbert_lut = Self::generate_hilbert_lut(device, queue);
        assets.texture_views.insert(
            SSAO.hilbert_lut_view,
            hilbert_lut.create_view(&Default::default()),
        );
        assets.textures.insert(SSAO.hilbert_lut, hilbert_lut);
    }

    fn prepare(
        &mut self,
        GpuScene { assets, .. }: &mut GpuScene,
        RenderContext {
            device,
            queue,
            targets,
            ..
        }: RenderContext,
    ) {
        let mut bf_config = DynamicGpuBuffer::new(BufferUsages::UNIFORM);
        bf_config.push(&SsaoConfig {
            texture_dim: targets.size,
            ..self.config
        });
        bf_config.write::<SsaoConfig>(device, queue);

        let compute_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("ssao_compute_bind_group"),
            layout: &assets.extra_layouts[&SSAO.ssao_compute_layout],
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &assets.texture_views[&DEPTH_PREPASS_TEXTURE.view],
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &assets.texture_views[&NORMAL_PREPASS_TEXTURE.view],
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(
                        &assets.texture_views[&SSAO.noisy_ssao_texture_view],
                    ),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: bf_config.binding::<SsaoConfig>().unwrap(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(&assets.samplers[&SSAO.ssao_sampler]),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: assets.camera_uniform.binding::<GpuCamera>().unwrap(),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: BindingResource::TextureView(
                        &assets.texture_views[&SSAO.hilbert_lut_view],
                    ),
                },
            ],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("ssao_bind_group"),
            layout: &assets.extra_layouts[&SSAO.ssao_layout],
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &assets.texture_views[&if self.denoise {
                            SSAO.ssao_texture_view
                        } else {
                            SSAO.noisy_ssao_texture_view
                        }],
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&assets.samplers[&SSAO.ssao_sampler]),
                },
            ],
        });

        assets
            .extra_bind_groups
            .insert(SSAO.ssao_compute_bind_group, compute_bind_group);
        assets
            .extra_bind_groups
            .insert(SSAO.ssao_bind_group, bind_group);

        let denoise_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("ssao_denoise_bind_group"),
            layout: &assets.extra_layouts[&SSAO.ssao_denoise_layout],
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: bf_config.entire_binding().unwrap(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &assets.texture_views[&SSAO.noisy_ssao_texture_view],
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&assets.samplers[&SSAO.ssao_sampler]),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(
                        &assets.texture_views[&SSAO.ssao_texture_view],
                    ),
                },
            ],
        });

        assets.extra_buffers.insert(SSAO.ssao_config, bf_config);
        assets
            .extra_bind_groups
            .insert(SSAO.ssao_denoise_bind_group, denoise_bind_group);
    }

    fn draw(
        &self,
        GpuScene { assets, .. }: &mut GpuScene,
        RenderContext {
            device,
            queue,
            targets,
            ..
        }: RenderContext,
    ) {
        let mut command_encoder = device.create_command_encoder(&Default::default());

        {
            let mut pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("ssao_pass"),
                ..Default::default()
            });

            pass.set_pipeline(self.compute_pipeline.as_ref().unwrap());
            pass.set_bind_group(
                0,
                &assets.extra_bind_groups[&SSAO.ssao_compute_bind_group],
                &[],
            );
            pass.dispatch_workgroups(
                targets.size.x.div_ceil(Self::SSAO_WORKGROUP_SIZE),
                targets.size.y.div_ceil(Self::SSAO_WORKGROUP_SIZE),
                1,
            );
        }

        {
            let mut pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("ssao_denoise_pass"),
                ..Default::default()
            });

            pass.set_pipeline(self.denoise_pipeline.as_ref().unwrap());
            pass.set_bind_group(
                0,
                &assets.extra_bind_groups[&SSAO.ssao_denoise_bind_group],
                &[],
            );

            pass.dispatch_workgroups(
                targets.size.x.div_ceil(Self::SSAO_WORKGROUP_SIZE),
                targets.size.y.div_ceil(Self::SSAO_WORKGROUP_SIZE),
                1,
            );
        }

        queue.submit([command_encoder.finish()]);
    }
}
