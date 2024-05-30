use std::collections::HashMap;

use uuid::Uuid;
use wgpu::{BindGroup, BindGroupLayout, BufferUsages};

use crate::{
    render::{
        resource::{DynamicGpuBuffer, GpuTexture, CAMERA_UUID, DIR_LIGHT_UUID},
        Transferable,
    },
    scene::{entity::Light, Scene},
    WgpuRenderer,
};

#[derive(Default)]
pub struct GpuScene {
    /// For camera and lights storage buffers, uuids are constants.

    /// For material uniform buffers, uuids are their type ids.
    /// For mesh vertex buffers, uuids are the corresponding mesh ids.
    pub buffers: HashMap<Uuid, DynamicGpuBuffer>,
    pub textures: HashMap<Uuid, GpuTexture>,

    /// For material bind groups, uuids are their individual uuids.
    pub bind_groups: HashMap<Uuid, BindGroup>,

    /// For material layouts, uuids are their type ids.
    pub layouts: HashMap<Uuid, BindGroupLayout>,
}

impl GpuScene {
    pub fn sync(&mut self, scene: &mut Scene, renderer: &WgpuRenderer) {
        let mut bf_camera = DynamicGpuBuffer::new(BufferUsages::UNIFORM);
        bf_camera.push(&scene.camera.transfer(renderer));
        bf_camera.write(&renderer.device, &renderer.queue);
        self.buffers.insert(CAMERA_UUID, bf_camera);

        let mut bf_dir_lights = DynamicGpuBuffer::new(BufferUsages::STORAGE);
        for light in &scene.lights {
            match light {
                Light::Directional(l) => bf_dir_lights.push(&l.transfer(renderer)),
            };
        }

        bf_dir_lights.write(&renderer.device, &renderer.queue);
        self.buffers.insert(DIR_LIGHT_UUID, bf_dir_lights);

        scene.meshes.drain().for_each(|(uuid, mesh)| {
            self.buffers.insert(uuid, mesh.transfer(renderer));
        });

        scene.materials.drain().for_each(|(uuid, (material, ty))| {
            if !self.layouts.contains_key(&ty) {
                self.layouts
                    .insert(ty, material.bind_group_layout(renderer));
            }
            material.create_bind_group(renderer, self, uuid);
        });
    }
}
