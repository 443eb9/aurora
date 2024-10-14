use std::any::TypeId;

use aurora_core::{
    render::{
        mesh::{CreateBindGroupLayout, Material},
        resource::DUMMY_2D_TEX,
        scene::{GpuAssets, MaterialInstanceId, MaterialTypeId, TextureId},
    },
    util::ext::{RgbToVec3, TypeIdAsUuid},
    WgpuRenderer,
};
use encase::ShaderType;
use glam::Vec3;
use palette::Srgb;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BufferBindingType, FilterMode, SamplerBindingType,
    SamplerDescriptor, ShaderStages, TextureSampleType, TextureViewDescriptor,
    TextureViewDimension,
};

use crate::node::TONY_MC_MAPFACE_LUT;

#[derive(Clone)]
pub struct PbrMaterial {
    pub base_color: Srgb,
    pub tex_base_color: Option<TextureId>,
    pub tex_normal: Option<TextureId>,
    pub roughness: f32,
    pub metallic: f32,
    pub reflectance: f32,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            base_color: Srgb::new(1., 1., 1.),
            tex_base_color: Default::default(),
            tex_normal: Default::default(),
            roughness: 1.,
            metallic: 0.,
            reflectance: 0.5,
        }
    }
}

#[derive(ShaderType)]
pub struct PbrMaterialUniform {
    pub base_color: Vec3,
    pub roughness: f32,
    pub metallic: f32,
    pub ior: f32,
}

impl CreateBindGroupLayout for PbrMaterial {
    fn create_layout(renderer: &WgpuRenderer, assets: &mut GpuAssets) {
        assets.material_layouts.insert(
            MaterialTypeId(TypeId::of::<Self>().to_uuid()),
            renderer
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("pbr_material_layout"),
                    entries: &[
                        // Material Uniform
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Uniform,
                                has_dynamic_offset: true,
                                min_binding_size: Some(PbrMaterialUniform::min_size()),
                            },
                            count: None,
                        },
                        // tex_base_color
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Texture {
                                sample_type: TextureSampleType::Float { filterable: true },
                                view_dimension: TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // tex_normal
                        BindGroupLayoutEntry {
                            binding: 2,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Texture {
                                sample_type: TextureSampleType::Float { filterable: true },
                                view_dimension: TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // Sampler
                        BindGroupLayoutEntry {
                            binding: 3,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Sampler(SamplerBindingType::Filtering),
                            count: None,
                        },
                        // LUT
                        BindGroupLayoutEntry {
                            binding: 4,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Texture {
                                sample_type: TextureSampleType::Float { filterable: true },
                                view_dimension: TextureViewDimension::D3,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // LUT Sampler
                        BindGroupLayoutEntry {
                            binding: 5,
                            visibility: ShaderStages::FRAGMENT,
                            ty: BindingType::Sampler(SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                }),
        );
    }
}

impl Material for PbrMaterial {
    fn create_bind_group(
        &self,
        renderer: &WgpuRenderer,
        assets: &mut GpuAssets,
        material: MaterialInstanceId,
    ) {
        let Some(buffer) = assets
            .material_uniforms
            .get(&self.id())
            .and_then(|b| b.binding::<PbrMaterialUniform>())
        else {
            return;
        };

        let layout = assets.material_layouts.get(&self.id()).unwrap();
        let pbr_material_bind_group = renderer.device.create_bind_group(&BindGroupDescriptor {
            label: Some("pbr_material_bind_group"),
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: buffer,
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &assets.textures[&self.tex_base_color.unwrap_or(DUMMY_2D_TEX)]
                            .create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(
                        &assets.textures[&self.tex_normal.unwrap_or(DUMMY_2D_TEX)]
                            .create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&renderer.device.create_sampler(
                        &SamplerDescriptor {
                            mag_filter: FilterMode::Linear,
                            min_filter: FilterMode::Linear,
                            mipmap_filter: FilterMode::Linear,
                            ..Default::default()
                        },
                    )),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(
                        &assets.textures[&TONY_MC_MAPFACE_LUT]
                            .create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::Sampler(&renderer.device.create_sampler(
                        &SamplerDescriptor {
                            mag_filter: FilterMode::Linear,
                            min_filter: FilterMode::Linear,
                            mipmap_filter: FilterMode::Linear,
                            ..Default::default()
                        },
                    )),
                },
            ],
        });

        assets
            .material_bind_groups
            .insert(material, pbr_material_bind_group);
    }

    fn prepare(&self, _renderer: &WgpuRenderer, assets: &mut GpuAssets) -> u32 {
        let buffer = assets.material_uniforms.get_mut(&self.id()).unwrap();
        buffer.push(&PbrMaterialUniform {
            base_color: self.base_color.into_linear().to_vec3(),
            roughness: self.roughness,
            metallic: self.metallic,
            ior: self.reflectance,
        })
    }
}
