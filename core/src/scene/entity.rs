use glam::{Mat4, Quat, Vec3};
use palette::Srgb;
use uuid::Uuid;

use crate::scene::resource::Mesh;

#[derive(Debug, Default, Clone, Copy)]
pub struct Camera {
    pub transform: Transform,
    pub projection: CameraProjection,
    /// Camera exposure in EV100
    pub exposure: Exposure,
}

pub enum Light {
    Directional(DirectionalLight),
    Point(PointLight),
    Spot(SpotLight),
    Area(AreaLight),
}

#[derive(Debug, Default, Clone, Copy)]
pub struct DirectionalLight {
    pub transform: Transform,
    pub color: Srgb,
    pub intensity: f32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PointLight {
    pub transform: Transform,
    pub color: Srgb,
    pub intensity: f32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SpotLight {
    pub transform: Transform,
    pub color: Srgb,
    pub intensity: f32,
    pub inner_angle: f32,
    pub outer_angle: f32,
}

#[derive(Debug, Default, Clone)]
pub struct AreaLight {
    pub mesh: Mesh,
    pub color: Srgb,
    pub intensity: f32,
    pub texture: Option<Uuid>,
}

#[derive(Debug, Clone, Copy)]
pub struct StaticMesh {
    pub mesh: Uuid,
    pub material: Uuid,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
}

macro_rules! impl_local_axis {
    ($method_pos: ident, $method_neg: ident, $axis_pos: ident, $axis_neg: ident) => {
        #[inline]
        pub fn $method_pos(&self) -> Vec3 {
            self.rotation * Vec3::$axis_pos
        }

        #[inline]
        pub fn $method_neg(&self) -> Vec3 {
            self.rotation * Vec3::$axis_neg
        }
    };
}

macro_rules! impl_rotation {
    ($method: ident, $rot_method: ident, $quat_method: ident) => {
        #[inline]
        pub fn $method(&mut self, angle: f32) {
            self.$rot_method(Quat::$quat_method(angle));
        }
    };
}

impl Transform {
    #[inline]
    pub fn transform_point(&self, p: Vec3) -> Vec3 {
        self.compute_matrix().mul_vec4(p.extend(0.)).truncate()
    }

    #[inline]
    pub fn compute_matrix(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation, self.translation)
    }

    #[inline]
    pub fn local_move(&mut self, delta: Vec3) {
        self.translation += self.rotation.inverse().mul_vec3(delta);
    }

    #[inline]
    pub fn rotate(&mut self, rotation: Quat) {
        self.rotation = rotation * self.rotation;
    }

    impl_rotation!(rotate_x, rotate, from_rotation_x);
    impl_rotation!(rotate_y, rotate, from_rotation_y);
    impl_rotation!(rotate_z, rotate, from_rotation_z);

    #[inline]
    pub fn local_rotate(&mut self, rotation: Quat) {
        self.rotation *= rotation;
    }

    impl_rotation!(local_rotate_x, local_rotate, from_rotation_x);
    impl_rotation!(local_rotate_y, local_rotate, from_rotation_y);
    impl_rotation!(local_rotate_z, local_rotate, from_rotation_z);

    impl_local_axis!(local_x, local_neg_x, X, NEG_X);
    impl_local_axis!(local_y, local_neg_y, Y, NEG_Y);
    impl_local_axis!(local_z, local_neg_z, Z, NEG_Z);
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
    pub fn symmetric(width: f32, height: f32, near: f32, far: f32) -> Self {
        Self {
            left: -width,
            right: width,
            bottom: -height,
            top: height,
            near,
            far,
        }
    }

    #[inline]
    pub fn compute_matrix(&self) -> Mat4 {
        Mat4::orthographic_rh(
            self.left,
            self.right,
            self.bottom,
            self.top,
            self.near,
            self.far,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Exposure {
    pub ev100: f32,
}

impl Default for Exposure {
    fn default() -> Self {
        Self { ev100: 9.7 }
    }
}

impl Exposure {
    pub fn from_physical(aperture: f32, shutter_speed: f32, sensitivity: f32) -> Self {
        Self {
            ev100: (aperture * aperture * 100. / shutter_speed / sensitivity).log2(),
        }
    }
}
