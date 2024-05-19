use aurora_derive::ShaderData;

use bytemuck::{Pod, Zeroable};

use glam::{EulerRot, Mat4, Vec3, Vec4};

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, Device,
};

use crate::{
    render::ShaderData,
    scene::{
        component::Mesh,
        entity::{Camera, DirectionalLight},
    },
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
            view: value.transform.compute_matrix(),
            proj: value.projection.compute_matrix(),
        }
    }
}

#[derive(ShaderData, Pod, Zeroable, Debug, Clone, Copy)]
#[repr(C)]
pub struct GpuDirectionalLight {
    pub position: Vec4,
    pub rotation: Vec4,
    pub color: Vec4,
}

impl From<DirectionalLight> for GpuDirectionalLight {
    fn from(value: DirectionalLight) -> Self {
        Self {
            position: value.transform.translation.extend(0.),
            rotation: Vec3::from(value.transform.rotation.to_euler(EulerRot::XYZ)).extend(0.),
            color: value.color.to_linear_rgba().into(),
        }
    }
}

pub struct GpuMesh {
    pub vertex_buffer: Buffer,
    pub vertex_count: u32,
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
        }
    }
}
