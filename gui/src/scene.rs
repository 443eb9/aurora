use aurora_core::render::helper::{Camera, Transform};
use glam::{EulerRot, Quat, Vec2, Vec3};

use winit::{
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
};

pub struct CameraConfig {
    pub tranl_sensi: f32,
    pub rot_sensi: Vec2,
    pub move_smoothness: f32,
    pub rot_smoothness: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            tranl_sensi: 2.,
            rot_sensi: Vec2::splat(50.),
            move_smoothness: 20.,
            rot_smoothness: 20.,
        }
    }
}

pub struct ControllableCamera {
    pub camera: Camera,
    target_camera: Transform,
    current_vel: Vec3,
    on_rotate: bool,
    pub config: CameraConfig,
}

impl ControllableCamera {
    pub fn new(camera: Camera, config: CameraConfig) -> Self {
        Self {
            target_camera: camera.transform,
            camera,
            current_vel: Vec3::ZERO,
            on_rotate: false,
            config,
        }
    }

    pub fn keyboard_control(&mut self, key: KeyCode, state: ElementState) {
        let t = match state {
            ElementState::Pressed => 1.,
            ElementState::Released => 0.,
        };

        match key {
            KeyCode::KeyW => self.current_vel.z = self.config.tranl_sensi * -t,
            KeyCode::KeyS => self.current_vel.z = self.config.tranl_sensi * t,
            KeyCode::KeyA => self.current_vel.x = self.config.tranl_sensi * -t,
            KeyCode::KeyD => self.current_vel.x = self.config.tranl_sensi * t,
            KeyCode::KeyQ => self.current_vel.y = self.config.tranl_sensi * -t,
            KeyCode::KeyE => self.current_vel.y = self.config.tranl_sensi * t,
            _ => {}
        }
    }

    pub fn update(&mut self, delta: f32) {
        self.target_camera.translation += self
            .camera
            .transform
            .rotation
            .mul_vec3(self.current_vel * self.config.tranl_sensi * delta);

        self.camera.transform.translation = self.camera.transform.translation.lerp(
            self.target_camera.translation,
            self.config.move_smoothness * delta,
        );
        self.camera.transform.rotation = self.camera.transform.rotation.lerp(
            self.target_camera.rotation,
            self.config.rot_smoothness * delta,
        );
    }

    pub fn mouse_control(&mut self, button: MouseButton, state: ElementState) {
        let t = match state {
            ElementState::Pressed => true,
            ElementState::Released => false,
        };

        match button {
            MouseButton::Left => {
                self.on_rotate = t;
            }
            _ => {}
        }
    }

    pub fn mouse_move(&mut self, offset: Vec2, delta: f32) {
        if self.on_rotate {
            let (mut yaw, mut pitch, _) = self.target_camera.rotation.to_euler(EulerRot::YXZ);
            yaw -= (offset.x * delta * self.config.rot_sensi.x).to_radians();
            pitch -= (offset.y * delta * self.config.rot_sensi.y).to_radians();
            pitch = pitch.clamp(-1.54, 1.54);
            self.target_camera.rotation =
                Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_axis_angle(Vec3::X, pitch);
        }
    }
}
