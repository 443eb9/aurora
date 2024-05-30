use std::collections::HashMap;

use uuid::Uuid;

use crate::scene::{
    entity::{Camera, Light, StaticMesh},
    resource::{Material, Mesh},
};

pub mod entity;
pub mod resource;

#[derive(Default)]
pub struct Scene {
    pub camera: Camera,
    pub lights: Vec<Light>,
    pub static_meshes: HashMap<Uuid, StaticMesh>,
    pub meshes: HashMap<Uuid, Mesh>,
    /// The Uuid as the key represents this specific material,
    /// and the Uuid as the value represents the type of this material.
    pub materials: HashMap<Uuid, (Box<dyn Material>, Uuid)>,
}

impl Scene {
    #[inline]
    pub fn insert_object(&mut self, object: impl SceneObject) -> Uuid {
        object.insert_self(self)
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
        uuid
    }
}

impl SceneObject for StaticMesh {
    fn insert_self(self, scene: &mut Scene) -> Uuid {
        let uuid = Uuid::new_v4();
        scene.static_meshes.insert(uuid, self);
        uuid
    }
}

pub trait MaterialObject: SceneObject {}
