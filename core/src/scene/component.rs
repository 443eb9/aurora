use std::path::Path;

use glam::{Mat4, Quat, Vec3};

use crate::render::Vertex;

#[derive(Debug, Clone, Copy, Default)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
}

impl Transform {
    pub fn transform_point(&self, p: Vec3) -> Vec3 {
        self.rotation.mul_vec3(p) + self.translation
    }

    pub fn compute_matrix(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation, self.translation)
    }

    pub fn local_move(&mut self, x: Vec3) {
        self.translation += self.rotation.mul_vec3(x);
    }

    pub fn local_rotate(&mut self, axis: Vec3, angle: f32) {
        self.rotation = Quat::from_axis_angle(axis, angle).mul_quat(self.rotation);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CameraProjection {
    Perspective(PerspectiveProjection),
    Orthographic(OrthographicProjection),
}

impl Default for CameraProjection {
    fn default() -> Self {
        Self::Perspective(PerspectiveProjection::default())
    }
}

impl CameraProjection {
    #[inline]
    pub fn compute_matrix(&self) -> Mat4 {
        match self {
            CameraProjection::Perspective(p) => p.compute_matrix(),
            CameraProjection::Orthographic(p) => p.compute_matrix(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PerspectiveProjection {
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for PerspectiveProjection {
    fn default() -> Self {
        Self {
            fov: std::f32::consts::FRAC_PI_4,
            aspect_ratio: 1.,
            near: 0.1,
            far: 1000.,
        }
    }
}

impl PerspectiveProjection {
    #[inline]
    pub fn compute_matrix(&self) -> Mat4 {
        // Mat4::perspective_infinite_reverse_rh(self.fov, self.aspect_ratio, self.near)
        Mat4::perspective_rh(self.fov, self.aspect_ratio, self.near, self.far)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OrthographicProjection {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
}

impl OrthographicProjection {
    #[inline]
    pub fn compute_matrix(&self) -> Mat4 {
        Mat4::orthographic_rh(
            self.left,
            self.right,
            self.bottom,
            self.top,
            // Swap to reverse depth
            self.far,
            self.near,
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
}

impl Mesh {
    pub fn from_obj(path: impl AsRef<Path>) -> Self {
        let mut source = Vec::new();
        std::io::Read::read_to_end(&mut std::fs::File::open(path).unwrap(), &mut source).unwrap();
        let obj = obj::ObjData::load_buf(&source[..]).unwrap();

        let mut vertices = Vec::new();
        for object in obj.objects {
            for group in object.groups {
                vertices.clear();
                for poly in group.polys {
                    for end_index in 2..poly.0.len() {
                        for &index in &[0, end_index - 1, end_index] {
                            let obj::IndexTuple(position_id, Some(_texture_id), Some(normal_id)) =
                                poly.0[index]
                            else {
                                unreachable!()
                            };

                            vertices.push(Vertex {
                                position: obj.position[position_id].into(),
                                normal: obj.normal[normal_id].into(),
                            });
                        }
                    }
                }
            }
        }

        Self { vertices }
    }
}
