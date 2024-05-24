use std::{
    f32::consts::FRAC_PI_4,
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};

use aurora_core::{
    color::SrgbaColor,
    node::{AuroraRenderFlow, DepthPassNode, PbrNode},
    render::RenderTargets,
    scene::{
        component::{CameraProjection, Mesh, PerspectiveProjection, Transform},
        entity::{Camera, DirectionalLight, Light},
        render::GpuScene,
        Scene,
    },
    wgpu::{TextureFormat, TextureUsages},
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
    renderer: WgpuSurfaceRenderer<'w>,
    window: Arc<Window>,
    dim: UVec2,

    main_camera: Arc<Mutex<ControllableCamera>>,
    scene: Scene,
    gpu_scene: GpuScene,

    flow: AuroraRenderFlow,
    last_draw: Instant,
    delta: f32,
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
                rotation: Quat::from_euler(EulerRot::XYZ, 1., 1., 1.),
                ..Default::default()
            },
            ..Default::default()
        }));
        scene.meshes.push(Mesh::from_obj("assets/cube.obj"));
        // scene.meshes.push(Mesh::from_obj("assets/plane.obj"));

        let mut gpu_scene = GpuScene::new(
            &scene,
            SrgbaColor {
                r: 0.,
                g: 0.,
                b: 0.,
                a: 0.,
            },
            renderer.device(),
        );
        gpu_scene.write_scene(renderer.device(), renderer.queue());

        let mut flow = AuroraRenderFlow::default();
        flow.add(
            "pbr".into(),
            Box::new(PbrNode::new(TextureFormat::Bgra8UnormSrgb)),
        );
        // flow.add(
        //     "depth_pass".into(),
        //     Box::new(DepthPassNode::new(TextureFormat::Bgra8UnormSrgb)),
        // );
        flow.build(renderer.device(), None);

        Self {
            renderer,
            window,
            dim,

            scene,
            gpu_scene,
            flow,

            main_camera: Arc::new(Mutex::new(main_camera)),

            delta: 0.,
            last_draw: Instant::now(),
        }
    }

    pub fn run(mut self, event_loop: EventLoop<()>) {
        let window = self.window.clone();
        let main_camera = self.main_camera.clone();
        let mut delta = 0.;

        thread::spawn(move || loop {
            let start = std::time::Instant::now();

            window.request_redraw();
            main_camera.lock().unwrap().update(delta);

            delta = start.elapsed().as_secs_f32();
        });

        event_loop.run_app(&mut self).unwrap();
    }

    pub fn handle_keyboard(&mut self, key: KeyCode, state: ElementState) {
        match key {
            KeyCode::F12 => pollster::block_on(self.take_screenshot()),
            _ => {}
        }

        let Ok(mut main_camera) = self.main_camera.lock() else {
            return;
        };
        main_camera.keyboard_control(key, state);
    }

    pub async fn take_screenshot(&mut self) {
        let mut flow = AuroraRenderFlow::default();
        flow.add(
            "pbr".into(),
            Box::new(PbrNode::new(TextureFormat::Rgba8Unorm)),
        );
        flow.add(
            "depth_pass".into(),
            Box::new(DepthPassNode::new(TextureFormat::Rgba8Unorm)),
        );

        flow.build(self.renderer.device(), None);
        flow.prepare(
            self.renderer.device(),
            &self.renderer.targets(),
            Some(&self.gpu_scene),
        );

        let (screenshot, screenshot_view) = aurora_core::utils::create_texture(
            self.renderer.device(),
            self.dim.extend(1),
            TextureFormat::Rgba8Unorm,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        );

        let (_depth, depth_view) = aurora_core::utils::create_texture(
            self.renderer.device(),
            self.dim.extend(1),
            TextureFormat::Depth32Float,
            TextureUsages::RENDER_ATTACHMENT,
        );

        self.renderer.renderer().render(
            &RenderTargets {
                color: &screenshot_view,
                depth: Some(&depth_view),
            },
            Some(&self.gpu_scene),
            &flow,
        );

        aurora_core::utils::save_color_texture_as_image(
            "generated/screenshot.png",
            &screenshot,
            self.renderer.device(),
            self.renderer.queue(),
        )
        .await;
    }

    pub fn redraw(&mut self) {
        let Ok(main_camera) = self.main_camera.lock() else {
            return;
        };
        self.scene.camera = main_camera.camera;

        self.gpu_scene.update_camera(&self.scene);
        self.gpu_scene
            .write_scene(self.renderer.device(), self.renderer.queue());

        self.renderer.update_surface();

        self.flow.prepare(
            self.renderer.device(),
            &self.renderer.targets(),
            Some(&self.gpu_scene),
        );

        self.delta = self.last_draw.elapsed().as_secs_f32();
        self.last_draw = Instant::now();
        self.renderer.draw(Some(&self.gpu_scene), &self.flow);
        self.renderer.update_frame_counter();
        self.renderer.present();
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
            WindowEvent::RedrawRequested => self.redraw(),
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
                main_camera.mouse_control(button, state);
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
                main_camera.mouse_move(Vec2::new(delta.0 as f32, delta.1 as f32), self.delta);
            }
            _ => {}
        }
    }
}
