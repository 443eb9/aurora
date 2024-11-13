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
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BufferBindingType, BufferUsages, ComputePassDescriptor,
    ComputePipeline, ComputePipelineDescriptor, Extent3d, FilterMode, PipelineLayoutDescriptor,
    SamplerBindingType, SamplerDescriptor, ShaderStages, StorageTextureAccess, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureViewDescriptor,
    TextureViewDimension,
};

use crate::node::{DEPTH_PREPASS_TEXTURE, NORMAL_PREPASS_TEXTURE};

#[derive(ShaderType)]
pub struct SsaoConfig {
    pub texture_dim: UVec2,
    pub slices: u32,
    pub samples: u32,
}

pub struct Ssao {
    pub ssao_texture: TextureId,
    pub ssao_texture_view: TextureViewId,

    pub ssao_compute_layout: ExtraLayoutId,
    pub ssao_compute_bind_group: ExtraBindGroupId,
    pub ssao_compute_sampler: SamplerId,
    pub ssao_config: ExtraBufferId,

    pub ssao_layout: ExtraLayoutId,
    pub ssao_bind_group: ExtraBindGroupId,
    pub ssao_sampler: SamplerId,
}

pub const SSAO: Ssao = Ssao {
    ssao_texture: TextureId(Uuid::from_u128(4365164098645125120)),
    ssao_texture_view: TextureViewId(Uuid::from_u128(15633484787465123021548)),

    ssao_compute_layout: ExtraLayoutId(Uuid::from_u128(153014631045364165)),
    ssao_compute_bind_group: ExtraBindGroupId(Uuid::from_u128(843650453641368049)),
    ssao_compute_sampler: SamplerId(Uuid::from_u128(8674601848654309867435)),
    ssao_config: ExtraBufferId(Uuid::from_u128(646846350026484867986486997)),

    ssao_layout: ExtraLayoutId(Uuid::from_u128(436840684109684365013)),
    ssao_bind_group: ExtraBindGroupId(Uuid::from_u128(4887674863516530486789645)),
    ssao_sampler: SamplerId(Uuid::from_u128(1464060146365201068451)),
};

pub const SSAO_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba32Float;

#[derive(Default)]
pub struct SsaoNode {
    pipeline: Option<ComputePipeline>,
}

impl SsaoNode {
    pub const CONFIG: SsaoConfig = SsaoConfig {
        texture_dim: UVec2::ZERO,
        slices: 16,
        samples: 4,
    };
    pub const WORKGROUP_SIZE: u32 = 16;
}

impl RenderNode for SsaoNode {
    fn require_shader_defs(
        &self,
        shader_defs: &mut HashMap<String, ShaderDefValue>,
        _config_bits: u32,
    ) {
        shader_defs.insert(
            "WORKGROUP_SIZE".to_string(),
            ShaderDefValue::UInt(Self::WORKGROUP_SIZE),
        );
    }

    fn require_shader(&self) -> Option<(&'static [&'static str], &'static str)> {
        Some((
            &[
                include_str!("../shader/common/common_type.wgsl"),
                include_str!("../shader/math.wgsl"),
                include_str!("../shader/hash.wgsl"),
            ],
            include_str!("../shader/post_processing/ssao_compute.wgsl"),
        ))
    }

    fn build(
        &mut self,
        GpuScene { assets, .. }: &mut GpuScene,
        RenderContext {
            device,
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
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("ssao_pipeline_layout"),
            bind_group_layouts: &[&compute_layout],
            push_constant_ranges: &[],
        });

        self.pipeline = Some(device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("ssao_pipeline"),
            layout: Some(&pipeline_layout),
            module: node.shader.as_ref().unwrap(),
            entry_point: "main",
            compilation_options: Default::default(),
            cache: None,
        }));

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

        let compute_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("ssao_compute_sampler"),
            ..Default::default()
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("ssao_sampler"),
            ..Default::default()
        });

        assets
            .extra_layouts
            .insert(SSAO.ssao_compute_layout, compute_layout);
        assets.extra_layouts.insert(SSAO.ssao_layout, layout);
        assets.textures.insert(SSAO.ssao_texture, ssao_texture);
        assets
            .texture_views
            .insert(SSAO.ssao_texture_view, ssao_texture_view);
        assets
            .samplers
            .insert(SSAO.ssao_compute_sampler, compute_sampler);
        assets.samplers.insert(SSAO.ssao_sampler, sampler);
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
            ..Self::CONFIG
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
                        &assets.texture_views[&SSAO.ssao_texture_view],
                    ),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: bf_config.binding::<SsaoConfig>().unwrap(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(
                        &assets.samplers[&SSAO.ssao_compute_sampler],
                    ),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: assets.camera_uniform.binding::<GpuCamera>().unwrap(),
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
                        &assets.texture_views[&SSAO.ssao_texture_view],
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&assets.samplers[&SSAO.ssao_sampler]),
                },
            ],
        });

        assets.extra_buffers.insert(SSAO.ssao_config, bf_config);
        assets
            .extra_bind_groups
            .insert(SSAO.ssao_compute_bind_group, compute_bind_group);
        assets
            .extra_bind_groups
            .insert(SSAO.ssao_bind_group, bind_group);
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

            pass.set_pipeline(self.pipeline.as_ref().unwrap());
            pass.set_bind_group(
                0,
                &assets.extra_bind_groups[&SSAO.ssao_compute_bind_group],
                &[],
            );
            pass.dispatch_workgroups(
                targets.size.x.div_ceil(Self::WORKGROUP_SIZE),
                targets.size.y.div_ceil(Self::WORKGROUP_SIZE),
                1,
            );
        }

        queue.submit([command_encoder.finish()]);
    }
}
