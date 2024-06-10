use std::collections::HashMap;

use uuid::Uuid;
use wgpu::{BindGroup, BindGroupLayout, BufferUsages, Texture};

use crate::{
    render::{
        resource::{
            DynamicGpuBuffer, GpuDirectionalLight, GpuPointLight, GpuSpotLight, CAMERA_UUID,
            DIR_LIGHT_UUID, POINT_LIGHT_UUID, SPOT_LIGHT_UUID,
        },
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
        let mut bf_camera = DynamicGpuBuffer::new(BufferUsages::UNIFORM);
        bf_camera.push(&scene.camera.transfer(renderer));
        bf_camera.write(&renderer.device, &renderer.queue);
        self.assets.buffers.insert(CAMERA_UUID, bf_camera);

        let mut bf_dir_lights = DynamicGpuBuffer::new(BufferUsages::STORAGE);
        let mut bf_point_lights = DynamicGpuBuffer::new(BufferUsages::STORAGE);
        let mut bf_spot_lights = DynamicGpuBuffer::new(BufferUsages::STORAGE);

        for light in &scene.lights {
            match light {
                Light::Directional(l) => bf_dir_lights.push(&l.transfer(renderer)),
                Light::Point(l) => bf_point_lights.push(&l.transfer(renderer)),
                Light::Spot(l) => bf_spot_lights.push(&l.transfer(renderer)),
            };
        }

        bf_dir_lights.safe_write::<GpuDirectionalLight>(&renderer.device, &renderer.queue);
        bf_point_lights.safe_write::<GpuPointLight>(&renderer.device, &renderer.queue);
        bf_spot_lights.safe_write::<GpuSpotLight>(&renderer.device, &renderer.queue);

        self.assets.buffers.insert(DIR_LIGHT_UUID, bf_dir_lights);
        self.assets
            .buffers
            .insert(POINT_LIGHT_UUID, bf_point_lights);
        self.assets.buffers.insert(SPOT_LIGHT_UUID, bf_spot_lights);

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
