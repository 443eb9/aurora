use crate::{
    color::SrgbaColor,
    scene::component::{CameraProjection, Transform},
};

pub enum Light {
    Directional(DirectionalLight),
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Camera {
    pub transform: Transform,
    pub projection: CameraProjection,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DirectionalLight {
    pub transform: Transform,
    pub color: SrgbaColor,
}
