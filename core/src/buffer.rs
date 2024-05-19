use std::marker::PhantomData;

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindingResource, Buffer, BufferUsages, Device, Queue,
};

use crate::render::ShaderData;

macro_rules! impl_buffer {
    ($buf_ty: ty, $default_usage: expr) => {
        impl<T: ShaderData> Default for $buf_ty {
            fn default() -> Self {
                Self {
                    raw: Default::default(),
                    buffer: Default::default(),
                    changed: Default::default(),
                    usage: $default_usage,
                    marker: Default::default(),
                }
            }
        }

        impl<T: ShaderData> $buf_ty {
            #[inline]
            pub fn set(&mut self, data: Vec<u8>) {
                self.raw = data;
                self.changed = true;
            }

            #[inline]
            pub fn push(&mut self, data: &T) {
                self.raw.extend_from_slice(data.as_bytes());
                self.changed = true;
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

pub struct UniformBuffer<T: ShaderData> {
    raw: Vec<u8>,
    buffer: Option<Buffer>,
    changed: bool,
    usage: BufferUsages,
    marker: PhantomData<T>,
}
impl_buffer!(
    UniformBuffer<T>,
    BufferUsages::UNIFORM | BufferUsages::COPY_DST
);

pub struct StorageBuffer<T: ShaderData> {
    raw: Vec<u8>,
    buffer: Option<Buffer>,
    changed: bool,
    usage: BufferUsages,
    marker: PhantomData<T>,
}
impl_buffer!(
    StorageBuffer<T>,
    BufferUsages::STORAGE | BufferUsages::COPY_DST
);
