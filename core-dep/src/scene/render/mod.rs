use std::{collections::HashMap, sync::Arc};

use wgpu::{
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType, Device, Queue,
    ShaderStages,
};

use crate::{
    color::{LinearRgbaColor, SrgbaColor},
    render::{GpuBinding, ShaderData},
    resource::{
        buffer::{SceneBuffers, StorageBuffer, UniformBuffer},
        material::Material,
        GpuTexture, ResRef,
    },
    scene::{
        entity::Light,
        render::entity::{GpuCamera, GpuDirectionalLight, GpuMesh},
        Scene,
    },
};

pub mod entity;

pub struct GpuScene {
    pub clear_color: LinearRgbaColor,

    pub buffers: SceneBuffers,

    pub b_camera: GpuBinding,
    pub b_lights: GpuBinding,

    pub meshes: Vec<GpuMesh>,
    pub textures: HashMap<ResRef, Arc<GpuTexture>>,
    pub materials: HashMap<ResRef, Arc<dyn Material>>,
}

impl GpuScene {
    pub fn new(scene: &Scene, clear_color: SrgbaColor, device: &Device, queue: &Queue) -> Self {
        let mut directional_lights = StorageBuffer::default();
        scene.lights.iter().for_each(|light| {
            match light {
                Light::Directional(l) => directional_lights.push(&GpuDirectionalLight::from(*l)),
            };
        });

        let textures = scene
            .textures
            .iter()
            .map(|(i, t)| (*i, Arc::new(t.clone_to_gpu(device, queue))))
            .collect();
        let meshes = scene
            .meshes
            .iter()
            .map(|m| m.clone_to_gpu(device))
            .collect();

        let mut camera = UniformBuffer::default();
        camera.push(&GpuCamera::from(scene.camera));

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

        let mut buffers = SceneBuffers::default();
        buffers.insert::<GpuDirectionalLight>(directional_lights.into());
        buffers.insert::<GpuCamera>(camera.into());

        Self {
            clear_color: clear_color.to_linear_rgba(),

            buffers,

            b_camera: GpuBinding::new(camera_layout),
            b_lights: GpuBinding::new(lights_layout),

            meshes,
            textures,
            materials: scene.materials.clone(),
        }
    }

    pub fn write_scene(&mut self, device: &Device, queue: &Queue) {
        self.buffers.write(device, queue);

        let (Some(camera), Some(directional_lights)) = (
            self.buffers.get_uniform::<GpuCamera>(),
            self.buffers.get_uniform::<GpuDirectionalLight>(),
        ) else {
            return;
        };

        self.b_camera.bind(device, [camera.binding().unwrap()]);
        self.b_lights
            .bind(device, [directional_lights.binding().unwrap()]);
    }

    pub fn update_camera(&mut self, scene: &Scene) {
        let buffer = self.buffers.get_uniform_mut::<GpuCamera>().unwrap();
        buffer.clear();
        buffer.push(&GpuCamera::from(scene.camera));
    }
}
