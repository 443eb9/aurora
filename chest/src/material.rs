use std::any::Any;

use aurora_core::{
    render::{resource::DynamicGpuBuffer, scene::GpuScene, ShaderData, Transferable},
    scene::resource::Material,
    util::TypeIdAsUuid,
    WgpuRenderer,
};
use aurora_derive::{MaterialObject, ShaderData};
use glam::Vec4;
use palette::Srgb;
use uuid::Uuid;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BufferBindingType, BufferUsages, ShaderStages,
};

#[derive(MaterialObject)]
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
    fn bind_group_layout(&self, renderer: &WgpuRenderer) -> BindGroupLayout {
        renderer
            .device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
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
            })
    }

    fn create_bind_group(&self, renderer: &WgpuRenderer, scene: &mut GpuScene, uuid: Uuid) {
        let ty = self.type_id().to_uuid();
        let Some(buffer) = scene.buffers.get(&ty).and_then(|b| b.binding()) else {
            return;
        };
        let layout = scene.layouts.get(&ty).unwrap();

        scene.bind_groups.insert(
            uuid,
            renderer.device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: buffer,
                }],
            }),
        );
    }

    fn prepare(&self, renderer: &WgpuRenderer, scene: &mut GpuScene) -> u32 {
        let buffer = scene
            .buffers
            .entry(self.type_id().to_uuid())
            .or_insert_with(|| DynamicGpuBuffer::new(BufferUsages::UNIFORM));
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
