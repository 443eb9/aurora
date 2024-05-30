use std::{collections::HashMap, sync::Arc};

use crate::{
    resource::{material::Material, Mesh, ResRef, Texture},
    scene::entity::{Camera, Light},
};

pub mod component;
pub mod entity;
pub mod render;

#[derive(Default)]
pub struct Scene {
    pub camera: Camera,
    pub lights: Vec<Light>,
    pub meshes: Vec<Mesh>,
    pub textures: HashMap<ResRef, Arc<Texture>>,
    pub materials: HashMap<ResRef, Arc<dyn Material>>,
}
