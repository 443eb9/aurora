use encase::{internal::WriteInto, DynamicStorageBuffer, DynamicUniformBuffer, ShaderType};
use glam::{Mat4, Vec2, Vec3, Vec4};
use uuid::Uuid;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindingResource, Buffer, BufferBinding, BufferUsages, Device, Queue, TextureFormat,
    TextureView,
};

use crate::scene::entity::StaticMesh;

pub const CAMERA_UUID: Uuid = Uuid::from_u128(4514851245144087048541368740532463840);
pub const LIGHT_VIEW_UUID: Uuid = Uuid::from_u128(78964516340896416354635118974);
pub const POST_PROCESS_COLOR_LAYOUT_UUID: Uuid = Uuid::from_u128(374318654136541653489410561064);
pub const POST_PROCESS_DEPTH_LAYOUT_UUID: Uuid = Uuid::from_u128(887897413248965416140604016399654);
pub const LIGHTS_BIND_GROUP_UUID: Uuid = Uuid::from_u128(7897465198640598654089653401853401968);
pub const DIR_LIGHT_UUID: Uuid = Uuid::from_u128(50864540865401960354989784651053240851);
pub const POINT_LIGHT_UUID: Uuid = Uuid::from_u128(7901283699454486410056310);
pub const SPOT_LIGHT_UUID: Uuid = Uuid::from_u128(123941018541520225801649306164979476413);

pub const DUMMY_2D_TEX: Uuid = Uuid::from_u128(8674167498640649160513219685401);

pub struct RenderTargets {
    pub color_format: TextureFormat,
    pub color: TextureView,
    pub depth_format: Option<TextureFormat>,
    pub depth: Option<TextureView>,
}

pub struct DynamicGpuBuffer {
    raw: DynamicStorageBuffer<Vec<u8>>,
    buffer: Option<Buffer>,
    changed: bool,
    usage: BufferUsages,
}

impl DynamicGpuBuffer {
    pub fn new(usage: BufferUsages) -> Self {
        Self {
            raw: DynamicStorageBuffer::new(Vec::new()),
            buffer: None,
            changed: true,
            usage: usage | BufferUsages::COPY_DST,
        }
    }

    pub fn set(&mut self, data: Vec<u8>) {
        self.raw = DynamicStorageBuffer::new(data);
        self.changed = true;
    }

    pub fn push<E: ShaderType + WriteInto>(&mut self, data: &E) -> u32 {
        self.raw.write(data).unwrap() as u32
    }

    pub fn usage(&self) -> &BufferUsages {
        &self.usage
    }

    pub fn usage_mut(&mut self) -> &mut BufferUsages {
        self.changed = true;
        &mut self.usage
    }

    pub fn write(&mut self, device: &Device, queue: &Queue) {
        let capacity = self.buffer.as_ref().map(|b| b.size()).unwrap_or(0);
        let size = self.raw.as_ref().len() as u64;

        if capacity < size || (self.changed && size > 0) {
            self.buffer = Some(device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                usage: self.usage,
                contents: self.raw.as_ref(),
            }));
            self.changed = false;
        } else if let Some(buffer) = &self.buffer {
            queue.write_buffer(buffer, 0, self.raw.as_ref());
        }
    }

    /// Extend the buffer with dummy data then write it.
    ///
    /// This prevents from binding 0-sized buffer, which is not allowed.
    /// But it will increase the array length by 1, so make sure to subtract
    /// that in shader.
    pub fn safe_write<T: ShaderType>(&mut self, device: &Device, queue: &Queue) {
        self.raw
            .as_mut()
            .extend_from_slice(&vec![0; T::min_size().get() as usize]);
        self.write(device, queue);
    }

    pub fn clear(&mut self) {
        self.raw.as_mut().clear();
        self.raw.set_offset(0);
    }

    pub fn binding<E: ShaderType>(&self) -> Option<BindingResource> {
        Some(BindingResource::Buffer(BufferBinding {
            buffer: self.buffer()?,
            offset: 0,
            size: Some(E::min_size()),
        }))
    }

    pub fn entire_binding(&self) -> Option<BindingResource> {
        self.buffer.as_ref().map(|b| b.as_entire_binding())
    }

    pub fn buffer(&self) -> Option<&Buffer> {
        self.buffer.as_ref()
    }

    pub fn len_bytes(&self) -> usize {
        self.raw.as_ref().len()
    }

    pub fn len<E>(&self) -> Option<usize> {
        let stride = std::mem::size_of::<E>();
        let b = self.raw.as_ref();
        if b.len() % stride == 0 {
            Some(b.len() / stride)
        } else {
            None
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
    pub tangent: Vec4,
}

#[derive(Clone)]
pub struct RenderMesh {
    pub mesh: StaticMesh,
    pub offset: Option<u32>,
}

#[derive(ShaderType)]
pub struct GpuCamera {
    pub view: Mat4,
    pub proj: Mat4,
    pub position_ws: Vec3,
    pub exposure: f32,
}

#[derive(ShaderType)]
pub struct GpuDirectionalLight {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
}

#[derive(ShaderType)]
pub struct GpuPointLight {
    pub position: Vec3,
    pub color: Vec3,
    pub intensity: f32,
}

#[derive(ShaderType)]
pub struct GpuSpotLight {
    pub position: Vec3,
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub inner_angle: f32,
    pub outer_angle: f32,
}
