use std::collections::HashMap;

use uuid::Uuid;

use crate::scene::{
    entity::{Camera, Light, StaticMesh},
    resource::{Image, Material, Mesh},
};

pub mod entity;
pub mod resource;

pub enum AssetType {
    Mesh,
    Material,
    Image,
    StaticMesh,
}

pub enum AssetEvent {
    Added(Uuid, AssetType),
    Removed(Uuid, AssetType),
}

#[derive(Default)]
pub struct Scene {
    pub camera: Camera,
    pub lights: HashMap<Uuid, Light>,
    pub static_meshes: HashMap<Uuid, StaticMesh>,
    pub meshes: HashMap<Uuid, Mesh>,
    /// The Uuid as the key represents this specific material,
    /// and the Uuid as the value represents the type of this material.
    pub materials: HashMap<Uuid, (Box<dyn Material>, Uuid)>,
    pub images: HashMap<Uuid, Image>,
    pub asset_events: Vec<AssetEvent>,
}

impl Scene {
    #[inline]
    pub fn insert_object(&mut self, object: impl SceneObject) -> Uuid {
        object.insert_self(self)
    }

    #[inline]
    pub fn remove_object(&mut self, object: Uuid, ty: AssetType) {
        match ty {
            AssetType::Mesh => {
                self.meshes.remove(&object);
            }
            AssetType::Material => {
                self.materials.remove(&object);
            }
            AssetType::StaticMesh => {
                self.static_meshes.remove(&object);
            }
            AssetType::Image => {
                self.images.remove(&object);
            }
        }
        self.asset_events.push(AssetEvent::Removed(object, ty));
    }

    #[inline]
    pub fn add_mesh_object(&mut self, mesh: StaticMesh) -> Uuid {
        let uuid = Uuid::new_v4();
        self.static_meshes.insert(uuid, mesh);
        uuid
    }
}

pub trait SceneObject {
    /// Returns a uuid stands for that object.
    fn insert_self(self, scene: &mut Scene) -> Uuid;
}

impl SceneObject for Mesh {
    fn insert_self(self, scene: &mut Scene) -> Uuid {
        let uuid = Uuid::new_v4();
        scene.meshes.insert(uuid, self);
        scene
            .asset_events
            .push(AssetEvent::Added(uuid, AssetType::Mesh));
        uuid
    }
}

impl SceneObject for StaticMesh {
    fn insert_self(self, scene: &mut Scene) -> Uuid {
        let uuid = Uuid::new_v4();
        scene.static_meshes.insert(uuid, self);
        scene
            .asset_events
            .push(AssetEvent::Added(uuid, AssetType::StaticMesh));
        uuid
    }
}

impl SceneObject for Image {
    fn insert_self(self, scene: &mut Scene) -> Uuid {
        let uuid = Uuid::new_v4();
        scene.images.insert(uuid, self);
        scene
            .asset_events
            .push(AssetEvent::Added(uuid, AssetType::Image));
        uuid
    }
}

impl SceneObject for Light {
    fn insert_self(self, scene: &mut Scene) -> Uuid {
        let uuid = Uuid::new_v4();
        scene.lights.insert(uuid, self);
        uuid
    }
}

pub trait MaterialObject: SceneObject {}
