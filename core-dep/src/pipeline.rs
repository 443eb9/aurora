use wgpu::*;

use crate::{
    render::ShaderData,
    scene::render::entity::{GpuCamera, GpuDirectionalLight},
};

pub struct PbrPipeline {
    pub camera_layout: BindGroupLayout,
    pub lights_layout: BindGroupLayout,
    pub pipeline_layout: PipelineLayout,
}

impl PbrPipeline {
    pub fn new(device: &Device) -> Self {
        let camera_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("pbr_camera_layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: GpuCamera::min_binding_size(),
                },
                count: None,
            }],
        });

        let lights_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("pbr_lights_layout"),
            entries: &[
                // Directional
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: GpuDirectionalLight::min_binding_size(),
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("pbr_pipeline_layout"),
            bind_group_layouts: &[&camera_layout, &lights_layout],
            push_constant_ranges: &[],
        });

        Self {
            camera_layout,
            lights_layout,

            pipeline_layout,
        }
    }
}

pub struct DepthPassPipeline {
    pub sampler: Sampler,
    pub bind_group_layout: BindGroupLayout,
    pub pipeline_layout: PipelineLayout,
}

impl DepthPassPipeline {
    pub fn new(device: &Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("depth_pass_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("depth_pass_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        Self {
            sampler,
            bind_group_layout,
            pipeline_layout,
        }
    }
}
