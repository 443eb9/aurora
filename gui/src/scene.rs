use aurora_core::scene::{component::Transform, entity::Camera};

use glam::{Vec2, Vec3};

use winit::{
    event::{ElementState, MouseButton},
    keyboard::KeyCode,
};

pub struct CameraConfig {
    pub tranl_sensi: f32,
    pub rot_sensi: Vec2,
    pub smoothness: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            tranl_sensi: 2.,
            rot_sensi: Vec2::splat(3.),
            smoothness: 20.,
        }
    }
}

pub struct ControllableCamera {
    pub camera: Camera,
    target_camera: Transform,
    current_vel: Vec3,
    on_rotate: bool,
    mouse_delta: Vec2,
    pub config: CameraConfig,
}

impl ControllableCamera {
    pub fn new(camera: Camera, config: CameraConfig) -> Self {
        Self {
            target_camera: camera.transform,
            camera,
            current_vel: Vec3::ZERO,
            on_rotate: false,
            mouse_delta: Vec2::ZERO,
            config,
        }
    }

    pub fn keyboard_control(&mut self, key: KeyCode, state: ElementState) {
        let t = match state {
            ElementState::Pressed => 1.,
            ElementState::Released => 0.,
        };

        match key {
            KeyCode::KeyW => self.current_vel.z = self.config.tranl_sensi * t,
            KeyCode::KeyS => self.current_vel.z = self.config.tranl_sensi * -t,
            KeyCode::KeyA => self.current_vel.x = self.config.tranl_sensi * t,
            KeyCode::KeyD => self.current_vel.x = self.config.tranl_sensi * -t,
            KeyCode::KeyQ => self.current_vel.y = self.config.tranl_sensi * -t,
            KeyCode::KeyE => self.current_vel.y = self.config.tranl_sensi * t,
            _ => {}
        }
    }

    pub fn mouse_control(&mut self, button: MouseButton, state: ElementState) {
        let t = match state {
            ElementState::Pressed => true,
            ElementState::Released => false,
        };

        match button {
            MouseButton::Left => {
                self.mouse_delta = Vec2::ZERO;
                self.on_rotate = t;
            }
            _ => {}
        }
    }

    pub fn mouse_move(&mut self, delta: Vec2) {
        if self.on_rotate {
            self.mouse_delta = delta;
        }
    }

    pub fn update(&mut self, delta: f32) {
        self.target_camera
            .local_move(self.current_vel * self.config.tranl_sensi * delta);
        self.camera.transform.translation = self.camera.transform.translation.lerp(
            self.target_camera.translation,
            self.config.smoothness * delta,
        );

        if self.on_rotate {
            self.camera.transform.local_rotate(
                Vec3::Y,
                self.mouse_delta.x * delta * self.config.rot_sensi.x,
            );
            self.camera.transform.local_rotate(
                Vec3::X,
                self.mouse_delta.y * delta * self.config.rot_sensi.y,
            );
        }
    }
}
