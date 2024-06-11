use std::any::Any;

use aurora_core::{
    render::{resource::DUMMY_2D_TEX, scene::GpuAssets, Transferable},
    scene::resource::Material,
    util::ext::{RgbToVec3, TypeIdAsUuid},
    WgpuRenderer,
};
use aurora_derive::MaterialObject;
use encase::ShaderType;
use glam::Vec3;
use palette::Srgb;
use uuid::Uuid;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BufferBindingType, FilterMode, SamplerBindingType,
    SamplerDescriptor, ShaderStages, TextureSampleType, TextureViewDescriptor,
    TextureViewDimension,
};

use crate::node::TONY_MC_MAPFACE_LUT;

#[derive(MaterialObject, Clone)]
pub struct PbrMaterial {
    pub base_color: Srgb,
    pub tex_base_color: Option<Uuid>,
    pub tex_normal: Option<Uuid>,
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
            reflectance: 1.,
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

impl Material for PbrMaterial {
    fn create_layout(&self, renderer: &WgpuRenderer, assets: &mut GpuAssets) {
        assets.layouts.insert(
            self.type_id().to_uuid(),
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

    fn create_bind_group(&self, renderer: &WgpuRenderer, assets: &mut GpuAssets, uuid: Uuid) {
        let ty = self.type_id().to_uuid();
        let Some(buffer) = assets
            .buffers
            .get(&ty)
            .and_then(|b| b.binding::<PbrMaterialUniform>())
        else {
            return;
        };
        let layout = assets.layouts.get(&ty).unwrap();

        assets.bind_groups.insert(
            uuid,
            renderer.device.create_bind_group(&BindGroupDescriptor {
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
            }),
        );
    }

    fn prepare(&self, renderer: &WgpuRenderer, assets: &mut GpuAssets) -> u32 {
        let buffer = assets.buffers.get_mut(&self.type_id().to_uuid()).unwrap();
        buffer.push(&self.transfer(renderer))
    }
}

impl Transferable for PbrMaterial {
    type GpuRepr = PbrMaterialUniform;

    fn transfer(&self, _renderer: &WgpuRenderer) -> Self::GpuRepr {
        PbrMaterialUniform {
            base_color: self.base_color.into_linear().to_vec3(),
            roughness: self.roughness,
            metallic: self.metallic,
            ior: self.reflectance,
        }
    }
}
