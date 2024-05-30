use std::{any::TypeId, collections::HashMap};

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindingResource, Buffer, BufferUsages, Device, Queue,
};

use crate::render::ShaderData;

macro_rules! impl_buffer {
    ($buf_ty: ty, $default_usage: expr) => {
        impl Default for $buf_ty {
            fn default() -> Self {
                Self {
                    raw: Default::default(),
                    buffer: Default::default(),
                    changed: Default::default(),
                    usage: $default_usage,
                }
            }
        }

        impl $buf_ty {
            #[inline]
            pub fn set(&mut self, data: Vec<u8>) {
                self.raw = data;
                self.changed = true;
            }

            #[inline]
            pub fn push(&mut self, data: &impl ShaderData) -> u32 {
                let offset = self.raw.len() as u32;
                self.raw.extend_from_slice(data.as_bytes());
                self.changed = true;
                offset
            }

            #[inline]
            pub fn usage(&self) -> &BufferUsages {
                &self.usage
            }

            #[inline]
            pub fn usage_mut(&mut self) -> &mut BufferUsages {
                &mut self.usage
            }

            pub fn write(&mut self, device: &Device, queue: &Queue) {
                let cap = self.buffer.as_ref().map(wgpu::Buffer::size).unwrap_or(0);
                let size = self.raw.len() as u64;

                if self.changed || cap < size {
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

            #[inline]
            pub fn clear(&mut self) {
                self.raw.clear();
            }

            #[inline]
            pub fn binding(&self) -> Option<BindingResource> {
                self.buffer.as_ref().map(|b| b.as_entire_binding())
            }
        }
    };
}

pub struct UniformBuffer {
    raw: Vec<u8>,
    buffer: Option<Buffer>,
    changed: bool,
    usage: BufferUsages,
}
impl_buffer!(
    UniformBuffer,
    BufferUsages::UNIFORM | BufferUsages::COPY_DST
);

pub struct StorageBuffer {
    raw: Vec<u8>,
    buffer: Option<Buffer>,
    changed: bool,
    usage: BufferUsages,
}
impl_buffer!(
    StorageBuffer,
    BufferUsages::STORAGE | BufferUsages::COPY_DST
);

pub enum GpuBuffer {
    Uniform(UniformBuffer),
    Storage(StorageBuffer),
}

impl GpuBuffer {
    #[inline]
    pub fn uniform(&self) -> Option<&UniformBuffer> {
        match self {
            GpuBuffer::Uniform(u) => Some(u),
            GpuBuffer::Storage(_) => None,
        }
    }

    #[inline]
    pub fn uniform_mut(&mut self) -> Option<&mut UniformBuffer> {
        match self {
            GpuBuffer::Uniform(u) => Some(u),
            GpuBuffer::Storage(_) => None,
        }
    }

    #[inline]
    pub fn storage(&self) -> Option<&StorageBuffer> {
        match self {
            GpuBuffer::Uniform(_) => None,
            GpuBuffer::Storage(s) => Some(s),
        }
    }

    #[inline]
    pub fn storage_mut(&mut self) -> Option<&mut StorageBuffer> {
        match self {
            GpuBuffer::Uniform(_) => None,
            GpuBuffer::Storage(s) => Some(s),
        }
    }
}

impl From<UniformBuffer> for GpuBuffer {
    fn from(value: UniformBuffer) -> Self {
        Self::Uniform(value)
    }
}

impl From<StorageBuffer> for GpuBuffer {
    fn from(value: StorageBuffer) -> Self {
        Self::Storage(value)
    }
}

#[derive(Default)]
pub struct SceneBuffers {
    value: HashMap<TypeId, GpuBuffer>,
}

impl SceneBuffers {
    pub fn get_uniform<T: 'static>(&self) -> Option<&UniformBuffer> {
        self.value.get(&TypeId::of::<T>()).and_then(|b| b.uniform())
    }

    pub fn get_uniform_mut<T: 'static>(&mut self) -> Option<&mut UniformBuffer> {
        self.value
            .get_mut(&TypeId::of::<T>())
            .and_then(|b| b.uniform_mut())
    }

    pub fn get_or_insert_uniform<T: 'static>(&mut self) -> Option<&mut UniformBuffer> {
        self.value
            .entry(TypeId::of::<T>())
            .or_insert_with(|| UniformBuffer::default().into())
            .uniform_mut()
    }

    pub fn get_storage<T: 'static>(&self) -> Option<&StorageBuffer> {
        self.value.get(&TypeId::of::<T>()).and_then(|b| b.storage())
    }

    pub fn get_storage_mut<T: 'static>(&mut self) -> Option<&mut StorageBuffer> {
        self.value
            .get_mut(&TypeId::of::<T>())
            .and_then(|b| b.storage_mut())
    }

    pub fn get_or_insert_storage<T: 'static>(&mut self) -> Option<&mut StorageBuffer> {
        self.value
            .entry(TypeId::of::<T>())
            .or_insert_with(|| StorageBuffer::default().into())
            .storage_mut()
    }

    pub fn insert<T: 'static>(&mut self, buffer: GpuBuffer) {
        self.value.insert(TypeId::of::<T>(), buffer);
    }

    pub fn write(&mut self, device: &Device, queue: &Queue) {
        self.value.values_mut().for_each(|b| match b {
            GpuBuffer::Uniform(u) => u.write(device, queue),
            GpuBuffer::Storage(s) => s.write(device, queue),
        });
    }
}
