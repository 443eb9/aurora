use std::{collections::HashMap, rc::Rc};

use glam::{Mat3, Mat4, Quat, Vec3};
use uuid::Uuid;

use crate::{
    render::{
        mesh::Material,
        resource::{GpuCamera, GpuDirectionalLight, GpuPointLight, GpuSpotLight},
        scene::MaterialInstanceId,
    },
    util::cube::CUBE_MAP_FACES,
};

#[derive(Default)]
pub struct Scene {
    pub camera: Camera,
    pub directional_lights: HashMap<Uuid, GpuDirectionalLight>,
    pub point_lights: HashMap<Uuid, GpuPointLight>,
    pub spot_lights: HashMap<Uuid, GpuSpotLight>,
    pub materials: HashMap<MaterialInstanceId, Rc<dyn Material>>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Camera {
    pub transform: Transform,
    pub projection: CameraProjection,
    /// Camera exposure in EV100
    pub exposure: Exposure,
}

impl Into<GpuCamera> for Camera {
    fn into(self) -> GpuCamera {
        GpuCamera {
            view: self.transform.compute_matrix().inverse(),
            proj: self.projection.compute_matrix(),
            position_ws: self.transform.translation,
            exposure: self.exposure.ev100,
        }
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
            left: -width * 0.5,
            right: width * 0.5,
            bottom: -height * 0.5,
            top: height * 0.5,
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

impl GpuDirectionalLight {
    pub fn light_view(&self) -> GpuCamera {
        GpuCamera {
            view: Mat4::look_to_rh(Vec3::ZERO, self.direction, Vec3::Y).inverse(),
            proj: Mat4::orthographic_rh(-16., 16., -16., 16., 20., -20.),
            position_ws: Vec3::ZERO,
            exposure: 0.,
        }
    }
}

impl GpuPointLight {
    pub fn light_view(&self) -> [GpuCamera; 6] {
        let mut views = [Default::default(); 6];
        CUBE_MAP_FACES
            .into_iter()
            .enumerate()
            .for_each(|(face_index, face)| {
                let trans = Transform::default()
                    .looking_at(face.target, face.up)
                    .with_translation(self.position);
                views[face_index] = GpuCamera {
                    view: trans.compute_matrix().inverse(),
                    proj: Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1., 0.1, 20.),
                    position_ws: trans.translation,
                    exposure: 0.,
                };
            });
        views
    }
}

impl GpuSpotLight {
    pub fn light_view(&self) -> [GpuCamera; 6] {
        GpuPointLight {
            position: self.position,
            color: self.color,
            intensity: self.intensity,
            radius: 0.,
        }
        .light_view()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
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

    #[inline]
    pub fn with_translation(mut self, translation: Vec3) -> Self {
        self.translation = translation;
        self
    }

    #[inline]
    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    #[inline]
    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    #[inline]
    pub fn looking_at(mut self, target: Vec3, up: Vec3) -> Self {
        self.look_at(target, up);
        self
    }

    // From Bevy
    #[inline]
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        let back = Vec3::normalize(self.translation - target);
        let right = up.cross(back).normalize();
        let up = back.cross(right);
        self.rotation = Quat::from_mat3(&Mat3::from_cols(right, up, back));
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
