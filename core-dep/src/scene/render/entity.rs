use aurora_derive::ShaderData;

use bytemuck::{Pod, Zeroable};

use glam::{Mat4, Vec4};

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, Device,
};

use crate::{
    render::ShaderData,
    resource::{Mesh, ResRef},
    scene::entity::{Camera, DirectionalLight},
};

#[derive(ShaderData, Pod, Zeroable, Debug, Clone, Copy)]
#[repr(C)]
pub struct GpuCamera {
    pub view: Mat4,
    pub proj: Mat4,
}

impl From<Camera> for GpuCamera {
    fn from(value: Camera) -> Self {
        Self {
            view: Mat4::from_quat(value.transform.rotation)
                * Mat4::from_translation(value.transform.translation),
            proj: value.projection.compute_matrix(),
        }
    }
}

#[derive(ShaderData, Pod, Zeroable, Debug, Clone, Copy)]
#[repr(C)]
pub struct GpuDirectionalLight {
    pub position: Vec4,
    pub direction: Vec4,
    pub color: Vec4,
}

impl From<DirectionalLight> for GpuDirectionalLight {
    fn from(value: DirectionalLight) -> Self {
        Self {
            position: value.transform.translation.extend(0.),
            direction: value.transform.local_neg_z().extend(0.),
            color: value.color.to_linear_rgba().into(),
        }
    }
}

pub struct GpuMesh {
    pub vertex_buffer: Buffer,
    pub vertex_count: u32,
    pub material: ResRef,
}

impl Mesh {
    pub fn clone_to_gpu(&self, device: &Device) -> GpuMesh {
        GpuMesh {
            vertex_buffer: device.create_buffer_init(&BufferInitDescriptor {
                label: Some("gpu_mesh_vertex_buffer"),
                contents: &self
                    .vertices
                    .iter()
                    .flat_map(|v| v.as_bytes())
                    .map(|b| *b)
                    .collect::<Vec<_>>(),
                usage: BufferUsages::VERTEX,
            }),
            vertex_count: self.vertices.len() as u32,
            material: self.material,
        }
    }
}
