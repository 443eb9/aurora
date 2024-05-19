use crate::scene::{
    component::Mesh,
    entity::{Camera, Light},
};

pub mod component;
pub mod entity;
pub mod render;

#[derive(Default)]
pub struct Scene {
    pub camera: Camera,
    pub lights: Vec<Light>,
    pub meshes: Vec<Mesh>,
}
