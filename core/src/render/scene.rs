use std::collections::HashMap;

use uuid::Uuid;
use wgpu::{BindGroup, BindGroupLayout, BufferUsages, Sampler, Texture, TextureView};

use crate::render::{helper::Scene, mesh::StaticMesh, resource::DynamicGpuBuffer};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialInstanceId(pub Uuid);

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialTypeId(pub Uuid);

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshInstanceId(pub Uuid);

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureId(pub Uuid);

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureViewId(pub Uuid);

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SamplerId(pub Uuid);

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExtraLayoutId(pub Uuid);

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExtraBindGroupId(pub Uuid);

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExtraUniformBufferId(pub Uuid);

pub struct GpuAssets {
    pub camera_uniform: DynamicGpuBuffer,
    pub scene_desc_uniform: DynamicGpuBuffer,
    pub directional_light_uniforms: DynamicGpuBuffer,
    pub point_light_uniforms: DynamicGpuBuffer,
    pub spot_light_uniforms: DynamicGpuBuffer,
    pub material_uniforms: HashMap<MaterialTypeId, DynamicGpuBuffer>,
    pub vertex_buffers: HashMap<MeshInstanceId, (DynamicGpuBuffer, u32)>,
    pub extra_uniforms: HashMap<ExtraUniformBufferId, DynamicGpuBuffer>,

    pub textures: HashMap<TextureId, Texture>,
    pub texture_views: HashMap<TextureViewId, TextureView>,
    pub samplers: HashMap<SamplerId, Sampler>,

    pub common_bind_group: Option<BindGroup>,
    pub light_bind_group: Option<BindGroup>,
    pub material_bind_groups: HashMap<MaterialInstanceId, BindGroup>,
    pub extra_bind_groups: HashMap<ExtraBindGroupId, BindGroup>,

    pub common_layout: Option<BindGroupLayout>,
    pub lights_layout: Option<BindGroupLayout>,
    pub material_layouts: HashMap<MaterialTypeId, BindGroupLayout>,
    pub extra_layouts: HashMap<ExtraLayoutId, BindGroupLayout>,
}

impl Default for GpuAssets {
    fn default() -> Self {
        Self {
            camera_uniform: DynamicGpuBuffer::new(BufferUsages::UNIFORM),
            scene_desc_uniform: DynamicGpuBuffer::new(BufferUsages::UNIFORM),
            directional_light_uniforms: DynamicGpuBuffer::new(BufferUsages::STORAGE),
            point_light_uniforms: DynamicGpuBuffer::new(BufferUsages::STORAGE),
            spot_light_uniforms: DynamicGpuBuffer::new(BufferUsages::STORAGE),
            material_uniforms: Default::default(),
            vertex_buffers: Default::default(),
            textures: Default::default(),
            common_bind_group: Default::default(),
            light_bind_group: Default::default(),
            material_bind_groups: Default::default(),
            common_layout: Default::default(),
            lights_layout: Default::default(),
            material_layouts: Default::default(),
            extra_bind_groups: Default::default(),
            extra_layouts: Default::default(),
            texture_views: Default::default(),
            extra_uniforms: Default::default(),
            samplers: Default::default(),
        }
    }
}

#[derive(Default)]
pub struct GpuScene {
    pub original: Scene,
    pub assets: GpuAssets,
    pub static_meshes: Vec<StaticMesh>,
}
