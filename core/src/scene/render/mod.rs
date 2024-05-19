use wgpu::{
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType, Device, Queue,
    ShaderStages,
};

use crate::{
    buffer::{StorageBuffer, UniformBuffer},
    color::{LinearRgbaColor, SrgbaColor},
    render::{GpuBinding, ShaderData},
    scene::{
        entity::Light,
        render::entity::{GpuCamera, GpuDirectionalLight, GpuMesh},
        Scene,
    },
};

pub mod entity;

pub struct GpuScene {
    pub clear_color: LinearRgbaColor,

    pub camera: UniformBuffer<GpuCamera>,
    pub directional_lights: StorageBuffer<GpuDirectionalLight>,

    pub b_camera: GpuBinding,
    pub b_lights: GpuBinding,

    pub meshes: Vec<GpuMesh>,
}

impl GpuScene {
    pub fn new(scene: &Scene, clear_color: SrgbaColor, device: &Device) -> Self {
        let mut directional_lights = StorageBuffer::<GpuDirectionalLight>::default();
        scene.lights.iter().for_each(|light| match light {
            Light::Directional(l) => directional_lights.push(&(*l).into()),
        });

        let meshes = scene
            .meshes
            .iter()
            .map(|m| m.clone_to_gpu(device))
            .collect();

        let mut camera = UniformBuffer::default();
        camera.push(&scene.camera.into());

        let camera_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("camera_bind_group_layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: GpuCamera::min_binding_size(),
                },
                count: None,
            }],
        });

        let lights_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("lights_bind_group_layout"),
            entries: &[
                // Directional
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: GpuDirectionalLight::min_binding_size(),
                    },
                    count: None,
                },
            ],
        });

        Self {
            clear_color: clear_color.to_linear_rgba(),

            camera,
            directional_lights,

            b_camera: GpuBinding::new(camera_layout),
            b_lights: GpuBinding::new(lights_layout),

            meshes,
        }
    }

    pub fn write_scene(&mut self, device: &Device, queue: &Queue) {
        self.camera.write(device, queue);
        self.directional_lights.write(device, queue);

        self.b_camera.bind(device, [self.camera.binding().unwrap()]);
        self.b_lights
            .bind(device, [self.directional_lights.binding().unwrap()]);
    }

    pub fn update_camera(&mut self, scene: &Scene) {
        self.camera.clear();
        self.camera.push(&scene.camera.into());
    }
}
