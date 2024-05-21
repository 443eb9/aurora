use std::{
    f32::consts::FRAC_PI_4,
    sync::{Arc, Mutex},
    thread,
};

use aurora_core::{
    builtin_pipeline::{AuroraPipeline, PbrPipeline},
    color::SrgbaColor,
    scene::{
        component::{CameraProjection, Mesh, PerspectiveProjection, Transform},
        entity::{Camera, DirectionalLight, Light},
        render::GpuScene,
        Scene,
    },
    *,
};

use glam::{EulerRot, Quat, UVec2, Vec2, Vec3};

use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalSize, Size},
    event::{DeviceEvent, DeviceId, ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

use crate::scene::{CameraConfig, ControllableCamera};

pub struct Application<'w> {
    pub renderer: WgpuSurfaceRenderer<'w>,
    window: Arc<Window>,
    dim: UVec2,

    main_camera: Arc<Mutex<ControllableCamera>>,
    scene: Scene,
    gpu_scene: GpuScene,
    pipeline: PbrPipeline<'w>,
}

impl<'w> Application<'w> {
    pub async fn new(event_loop: &EventLoop<()>, dim: UVec2) -> Self {
        #[allow(deprecated)]
        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_inner_size(Size::Physical(PhysicalSize::new(dim.x, dim.y))),
                )
                .unwrap(),
        );

        let renderer = WgpuSurfaceRenderer::new(window.clone(), dim).await;

        let main_camera = ControllableCamera::new(
            Camera {
                transform: Transform {
                    translation: Vec3::new(0., 0., -10.),
                    ..Default::default()
                },
                projection: CameraProjection::Perspective(PerspectiveProjection {
                    aspect_ratio: dim.x as f32 / dim.y as f32,
                    fov: FRAC_PI_4,
                    near: 0.1,
                    far: 1000.,
                }),
            },
            CameraConfig::default(),
        );

        let mut scene = Scene {
            camera: main_camera.camera,
            ..Default::default()
        };

        scene.lights.push(Light::Directional(DirectionalLight {
            transform: Transform {
                translation: Vec3::new(10., 20., 0.),
                rotation: Quat::from_euler(EulerRot::XYZ, -1., -1.2, 1.),
            },
            ..Default::default()
        }));
        scene.meshes.push(Mesh::from_obj("assets/icosahedron.obj"));

        let mut gpu_scene = GpuScene::new(
            &scene,
            SrgbaColor {
                r: 0.,
                g: 0.,
                b: 0.,
                a: 0.,
            },
            renderer.renderer().device(),
        );
        gpu_scene.write_scene(renderer.renderer().device(), renderer.renderer().queue());

        let device = renderer.renderer().device();
        let mut pipeline = PbrPipeline::new(device, TextureFormat::Bgra8UnormSrgb);
        pipeline.build(device, Default::default());

        Self {
            renderer,
            window,
            dim,

            scene,
            gpu_scene,
            pipeline,

            main_camera: Arc::new(Mutex::new(main_camera)),
        }
    }

    pub fn run(&mut self, event_loop: EventLoop<()>) {
        let window = self.window.clone();
        let main_camera = self.main_camera.clone();
        let mut delta = 0.;

        thread::spawn(move || loop {
            let start = std::time::Instant::now();

            window.request_redraw();
            main_camera.lock().unwrap().update(delta);

            delta = start.elapsed().as_secs_f32();
        });

        event_loop.run_app(self).unwrap();
    }

    pub fn handle_keyboard(&self, key: KeyCode, state: ElementState) {
        let Ok(mut main_camera) = self.main_camera.lock() else {
            return;
        };
        main_camera.keyboard_control(key, state);

        match key {
            KeyCode::F12 => pollster::block_on(self.handle_screenshot()),
            _ => {}
        }
    }

    pub async fn handle_screenshot(&self) {
        let renderer = WgpuImageRenderer::new(self.dim).await;
        renderer.draw(Some(&self.gpu_scene), &self.pipeline).await;
        renderer.save_result("genearated/").await;
    }
}

impl<'w> ApplicationHandler for Application<'w> {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                let Ok(main_camera) = self.main_camera.lock() else {
                    return;
                };
                self.scene.camera = main_camera.camera;
                self.gpu_scene.update_camera(&self.scene);
                self.gpu_scene.write_scene(
                    self.renderer.renderer().device(),
                    self.renderer.renderer().queue(),
                );
                self.renderer.draw(Some(&self.gpu_scene), &self.pipeline);
                self.renderer.update_frame_counter();
            }
            WindowEvent::Resized(size) => self.renderer.resize(UVec2::new(size.width, size.height)),
            WindowEvent::CloseRequested => std::process::exit(0),
            WindowEvent::KeyboardInput {
                device_id: _,
                event,
                is_synthetic: _,
            } => match event.physical_key {
                PhysicalKey::Code(key) => self.handle_keyboard(key, event.state),
                PhysicalKey::Unidentified(_) => {}
            },
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                let Ok(mut main_camera) = self.main_camera.lock() else {
                    return;
                };
                main_camera.mouse_control(button, &state);
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                let Ok(mut main_camera) = self.main_camera.lock() else {
                    return;
                };
                main_camera.mouse_move(Vec2::new(delta.0 as f32, delta.1 as f32));
            }
            _ => {}
        }
    }
}
