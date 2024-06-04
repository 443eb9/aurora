use std::collections::HashMap;

use uuid::Uuid;
use wgpu::{BindGroup, BindGroupLayout, BufferUsages, Texture};

use crate::{
    render::{
        resource::{DynamicGpuBuffer, CAMERA_UUID, DIR_LIGHT_UUID},
        Transferable,
    },
    scene::{entity::Light, resource::Material, AssetEvent, AssetType, Scene},
    WgpuRenderer,
};

#[derive(Default)]
pub struct GpuAssets {
    /// For camera and lights storage buffers, uuids are constants.

    /// For material uniform buffers, uuids are their type ids.
    /// For mesh vertex buffers, uuids are the corresponding mesh ids.
    pub buffers: HashMap<Uuid, DynamicGpuBuffer>,
    pub textures: HashMap<Uuid, Texture>,

    /// For material bind groups, uuids are their individual uuids.
    pub bind_groups: HashMap<Uuid, BindGroup>,

    /// For material layouts, uuids are their type ids.
    pub layouts: HashMap<Uuid, BindGroupLayout>,
}

#[derive(Default)]
pub struct GpuScene {
    pub assets: GpuAssets,
    pub materials: HashMap<Uuid, Box<dyn Material>>,
}

impl GpuScene {
    pub fn sync(&mut self, scene: &mut Scene, renderer: &WgpuRenderer) {
        let bf_camera = self
            .assets
            .buffers
            .entry(CAMERA_UUID)
            .or_insert_with(|| DynamicGpuBuffer::new(BufferUsages::UNIFORM));
        bf_camera.clear();
        bf_camera.push(&scene.camera.transfer(renderer));
        bf_camera.write(&renderer.device, &renderer.queue);

        let bf_dir_lights = self
            .assets
            .buffers
            .entry(DIR_LIGHT_UUID)
            .or_insert_with(|| DynamicGpuBuffer::new(BufferUsages::STORAGE));
        bf_dir_lights.clear();
        for light in &scene.lights {
            match light {
                Light::Directional(l) => bf_dir_lights.push(&l.transfer(renderer)),
            };
        }

        bf_dir_lights.write(&renderer.device, &renderer.queue);

        scene.asset_events.drain(..).for_each(|ae| match ae {
            AssetEvent::Added(uuid, ty) => match ty {
                AssetType::Mesh => {
                    self.assets
                        .buffers
                        .insert(uuid, scene.meshes[&uuid].transfer(renderer));
                }
                AssetType::Material => {
                    let (material, ty) = &scene.materials[&uuid];
                    if !self.assets.layouts.contains_key(&ty) {
                        material.create_layout(renderer, &mut self.assets);
                    }
                    self.materials
                        .insert(uuid, dyn_clone::clone_box(material.as_ref()));
                }
                AssetType::Image => {
                    self.assets
                        .textures
                        .insert(uuid, scene.images[&uuid].transfer(renderer));
                }
                // Ignore
                AssetType::StaticMesh => {}
            },
            AssetEvent::Removed(uuid, ty) => match ty {
                AssetType::Mesh => {
                    self.assets.buffers.remove(&uuid);
                }
                AssetType::Material => {
                    self.assets.bind_groups.remove(&uuid);
                    self.materials.remove(&uuid);
                }
                AssetType::Image => {
                    self.assets.textures.remove(&uuid);
                }
                // Ignore
                AssetType::StaticMesh => {}
            },
        });

        renderer.queue.submit(None);
    }
}
