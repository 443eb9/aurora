use aurora_derive::ShaderData;
use glam::{Mat4, Vec2, Vec3, Vec4};
use uuid::Uuid;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindingResource, Buffer, BufferUsages, Device, Queue, TextureFormat, TextureView,
};

use crate::{render::ShaderData, scene::entity::StaticMesh};

pub const CAMERA_UUID: Uuid = Uuid::from_u128(4514851245144087048541368740532463840);
pub const POST_PROCESS_COLOR_LAYOUT_UUID: Uuid = Uuid::from_u128(374318654136541653489410561064);
pub const POST_PROCESS_DEPTH_LAYOUT_UUID: Uuid = Uuid::from_u128(887897413248965416140604016399654);
pub const LIGHTS_BIND_GROUP_UUID: Uuid = Uuid::from_u128(7897465198640598654089653401853401968);
pub const DIR_LIGHT_UUID: Uuid = Uuid::from_u128(50864540865401960354989784651053240851);

pub struct RenderTarget {
    pub color_format: TextureFormat,
    pub color: TextureView,
    pub depth_format: Option<TextureFormat>,
    pub depth: Option<TextureView>,
}

pub struct DynamicGpuBuffer {
    raw: Vec<u8>,
    buffer: Option<Buffer>,
    changed: bool,
    usage: BufferUsages,
}

impl DynamicGpuBuffer {
    pub fn new(usage: BufferUsages) -> Self {
        Self {
            raw: Vec::new(),
            buffer: None,
            changed: true,
            usage,
        }
    }

    pub fn set(&mut self, data: Vec<u8>) {
        self.raw = data;
        self.changed = true;
    }

    pub fn push(&mut self, data: &impl ShaderData) -> u32 {
        let offset = self.raw.len() as u32;
        self.raw.extend_from_slice(data.as_bytes());
        self.changed = true;
        offset
    }

    pub fn usage(&self) -> &BufferUsages {
        &self.usage
    }

    pub fn usage_mut(&mut self) -> &mut BufferUsages {
        &mut self.usage
    }

    pub fn write(&mut self, device: &Device, queue: &Queue) {
        let cap = self.buffer.as_ref().map(Buffer::size).unwrap_or(0);
        let size = self.raw.len() as u64;

        if cap < size || (self.changed && size > 0) {
            self.buffer = Some(device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: &self.raw,
                usage: self.usage,
            }));
            self.changed = false;
        } else if let Some(buffer) = &self.buffer {
            queue.write_buffer(&buffer, 0, &self.raw);
        }
    }

    pub fn clear(&mut self) {
        self.raw.clear();
    }

    pub fn binding(&self) -> Option<BindingResource> {
        self.buffer.as_ref().map(|b| b.as_entire_binding())
    }

    pub fn buffer(&self) -> Option<&Buffer> {
        self.buffer.as_ref()
    }

    pub fn len<E>(&self) -> Option<usize> {
        let stride = std::mem::size_of::<E>();
        if self.raw.len() % stride == 0 {
            Some(self.raw.len() / stride)
        } else {
            None
        }
    }
}

#[derive(ShaderData)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec3,
}

pub struct RenderMesh {
    pub mesh: StaticMesh,
    pub offset: Option<u32>,
}

#[derive(ShaderData)]
pub struct GpuCamera {
    pub view: Mat4,
    pub proj: Mat4,
}

#[derive(ShaderData)]
pub struct GpuDirectionalLight {
    pub position: Vec4,
    pub direction: Vec4,
    pub color: Vec4,
}
