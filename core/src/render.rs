use std::num::NonZeroU64;

use aurora_derive::ShaderData;
use bytemuck::{Pod, Zeroable};

use glam::Vec3;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource, Device,
};

pub trait ShaderData: Sized + Pod {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    #[inline]
    fn min_binding_size() -> Option<NonZeroU64> {
        Some(unsafe { NonZeroU64::new_unchecked(std::mem::size_of::<Self>() as u64) })
    }
}

pub struct GpuBinding {
    pub bind_group: Option<BindGroup>,
    pub layout: BindGroupLayout,
}

impl GpuBinding {
    pub fn new(layout: BindGroupLayout) -> Self {
        Self {
            bind_group: None,
            layout,
        }
    }

    pub fn bind<const N: usize>(&mut self, device: &Device, bindings: [BindingResource; N]) {
        self.bind_group = Some(
            device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &self.layout,
                entries: &bindings
                    .into_iter()
                    .enumerate()
                    .map(|(binding, resource)| BindGroupEntry {
                        binding: binding as u32,
                        resource,
                    })
                    .collect::<Vec<_>>(),
            }),
        );
    }
}

#[derive(ShaderData, Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
}
