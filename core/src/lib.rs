use std::cell::RefCell;

use wgpu::{
    Adapter, Device, DeviceDescriptor, Features, Instance, Limits, MemoryHints, Queue,
    RequestAdapterOptions, Texture, TextureDescriptor, TextureView,
};

pub mod render;
pub mod util;

pub struct WgpuRenderer {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
}

impl WgpuRenderer {
    pub async fn new(required_features: Option<Features>, required_limits: Option<Limits>) -> Self {
        let instance = Instance::default();
        let adapter = instance
            .request_adapter(&RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    required_features: required_features.unwrap_or_default(),
                    required_limits: required_limits.unwrap_or_default(),
                    memory_hints: MemoryHints::Performance,
                },
                None,
            )
            .await
            .unwrap();

        Self {
            instance,
            adapter,
            device,
            queue,
        }
    }
}

pub struct PostProcess<'a> {
    src: &'a TextureView,
    dst: &'a TextureView,
}

pub struct PostProcessChain {
    main_texture: RefCell<bool>,
    main_texture_a: Texture,
    main_view_a: TextureView,
    main_texture_b: Texture,
    main_view_b: TextureView,
}

impl PostProcessChain {
    pub fn new(device: &Device, desc: &TextureDescriptor) -> Self {
        let a = device.create_texture(desc);
        let b = device.create_texture(desc);

        Self {
            main_texture: RefCell::new(false),
            main_view_a: a.create_view(&Default::default()),
            main_texture_a: a,
            main_view_b: b.create_view(&Default::default()),
            main_texture_b: b,
        }
    }

    pub fn swap(&self) {
        self.main_texture.replace(self.main_texture());
    }

    pub fn main_texture(&self) -> bool {
        *self.main_texture.borrow()
    }

    pub fn current_texture(&self) -> &Texture {
        if self.main_texture() {
            &self.main_texture_a
        } else {
            &self.main_texture_b
        }
    }

    pub fn current_view(&self) -> &TextureView {
        if self.main_texture() {
            &self.main_view_a
        } else {
            &self.main_view_b
        }
    }

    pub fn another_texture(&self) -> &Texture {
        if !self.main_texture() {
            &self.main_texture_a
        } else {
            &self.main_texture_b
        }
    }

    pub fn another_view(&self) -> &TextureView {
        if !self.main_texture() {
            &self.main_view_a
        } else {
            &self.main_view_b
        }
    }
}
