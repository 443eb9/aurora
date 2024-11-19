use aurora_core::render::{
    flow::{NodeExtraData, RenderContext, RenderNode},
    resource::{DynamicGpuBuffer, Image},
    scene::{ExtraBindGroupId, ExtraBufferId, ExtraLayoutId, GpuScene, SamplerId, TextureId},
};
use encase::ShaderType;
use image::ImageFormat;
use uuid::Uuid;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BufferBindingType, BufferUsages, SamplerBindingType,
    SamplerDescriptor, ShaderStages, TextureSampleType, TextureViewDescriptor,
    TextureViewDimension,
};

#[derive(ShaderType)]
pub struct EnvironmentMappingConfig {
    pub intensity: f32,
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

#[derive(Default)]
pub struct EnvironmentMappingNode;

impl EnvironmentMappingNode {
    pub const CONFIG: EnvironmentMappingConfig = EnvironmentMappingConfig { intensity: 100. };
}

impl RenderNode for EnvironmentMappingNode {
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

        let env_map = Image::from_buffer(&std::fs::read(path).unwrap(), ImageFormat::Hdr, false)
            .to_cube_map(device, queue, &Default::default());

        let env_map_view = env_map.create_view(&TextureViewDescriptor {
            label: Some("environment_map_texture_view"),
            dimension: Some(TextureViewDimension::Cube),
            ..Default::default()
        });

        let env_map_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("environment_map_sampler"),
            ..Default::default()
        });

        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("environment_mapping_layout"),
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
                // Sampler
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                    count: None,
                },
                // Uniform
                BindGroupLayoutEntry {
                    binding: 2,
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
        bf_config.push(&Self::CONFIG);
        bf_config.write::<EnvironmentMappingConfig>(device, queue);

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("environment_mapping_bind_group"),
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&env_map_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&env_map_sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: bf_config.entire_binding().unwrap(),
                },
            ],
        });

        assets
            .extra_layouts
            .insert(ENV_MAPPING.env_mapping_layout, layout);
        assets
            .extra_bind_groups
            .insert(ENV_MAPPING.env_mapping_bind_group, bind_group);
        assets.textures.insert(ENV_MAPPING.env_map_texture, env_map);
        assets
            .samplers
            .insert(ENV_MAPPING.env_map_sampler, env_map_sampler);
        assets
            .extra_buffers
            .insert(ENV_MAPPING.env_map_config, bf_config);
    }
}
