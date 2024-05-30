use std::sync::Arc;

use aurora_derive::ShaderData;
use bytemuck::{Pod, Zeroable};
use glam::Vec4;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Device, SamplerBindingType, ShaderStages, TextureSampleType,
    TextureViewDimension,
};

use crate::{
    color::SrgbaColor,
    resource::{buffer::SceneBuffers, Texture},
};

pub trait Material {
    fn bind_group_layout(&self, device: &Device) -> BindGroupLayout;
    fn bind(
        &self,
        device: &Device,
        layout: &BindGroupLayout,
        buffers: &SceneBuffers,
    ) -> Option<BindGroup>;
    fn push_uniform(&self, buffers: &mut SceneBuffers) -> u32;
}

pub struct StandardMaterial {
    pub base_color: SrgbaColor,
    pub tex_base_color: Option<Arc<Texture>>,
    pub metallic: f32,
    pub roughness: f32,
}

#[derive(ShaderData, Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct StandardMaterialUniform {
    pub base_color: Vec4,
    pub metallic: f32,
    pub roughness: f32,
    pub _padding: u64,
}

impl Material for StandardMaterial {
    fn bind_group_layout(&self, device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
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
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }

    fn push_uniform(&self, buffers: &mut SceneBuffers) -> u32 {
        buffers
            .get_or_insert_uniform::<Self>()
            .unwrap()
            .push(&StandardMaterialUniform {
                base_color: self.base_color.to_linear_rgba().into(),
                metallic: self.metallic,
                roughness: self.roughness,
                _padding: 0,
            })
    }

    fn bind(
        &self,
        device: &Device,
        layout: &BindGroupLayout,
        buffers: &SceneBuffers,
    ) -> Option<BindGroup> {
        let Some(buffer) = buffers
            .get_uniform::<StandardMaterialUniform>()
            .and_then(|b| b.binding())
        else {
            return None;
        };

        Some(device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer,
            }],
        }))
    }
}
