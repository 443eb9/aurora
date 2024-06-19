use wgpu::{
    Adapter, Device, DeviceDescriptor, Features, Instance, Limits, Queue, RequestAdapterOptions,
};

pub mod render;
pub mod scene;
pub mod util;

pub struct WgpuRenderer {
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
}

impl WgpuRenderer {
    pub async fn new() -> Self {
        let instance = Instance::default();
        let adapter = instance
            .request_adapter(&RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
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
