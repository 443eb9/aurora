use std::any::Any;

use aurora_core::{
    render::{scene::GpuAssets, ShaderData, Transferable},
    scene::resource::Material,
    util::TypeIdAsUuid,
    WgpuRenderer,
};
use aurora_derive::{MaterialObject, ShaderData};
use glam::Vec4;
use palette::Srgb;
use uuid::Uuid;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingType, BufferBindingType, ShaderStages,
};

#[derive(MaterialObject, Clone)]
pub struct PbrMaterial {
    pub base_color: Srgb,
    pub tex_base_color: Option<Uuid>,
    pub roughness: f32,
    pub metallic: f32,
}

#[derive(ShaderData)]
pub struct PbrMaterialUniform {
    pub base_color: Vec4,
    pub roughness: f32,
    pub metallic: f32,
}

impl Material for PbrMaterial {
    fn create_layout(&self, renderer: &WgpuRenderer, assets: &mut GpuAssets) {
        assets.layouts.insert(
            self.type_id().to_uuid(),
            renderer
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("pbr_material_layout"),
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: PbrMaterialUniform::min_binding_size(),
                        },
                        count: None,
                    }],
                }),
        );
    }

    fn create_bind_group(&self, renderer: &WgpuRenderer, assets: &mut GpuAssets, uuid: Uuid) {
        let ty = self.type_id().to_uuid();
        let Some(buffer) = assets.buffers.get(&ty).and_then(|b| b.binding()) else {
            return;
        };
        let layout = assets.layouts.get(&ty).unwrap();

        assets.bind_groups.insert(
            uuid,
            renderer.device.create_bind_group(&BindGroupDescriptor {
                label: Some("pbr_material_bind_group"),
                layout: &layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: buffer,
                }],
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
        let col = self.base_color.into_linear();
        PbrMaterialUniform {
            base_color: Vec4::new(col.red, col.green, col.blue, 1.),
            roughness: self.roughness,
            metallic: self.metallic,
        }
    }
}
