use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};

use aurora_chest::shader_defs::{PbrDiffuse, PbrSpecular};
use aurora_core::{
    render::{resource::RenderTargets, scene::GpuScene, ShaderDefEnum},
    scene::{
        entity::{Camera, CameraProjection, Exposure, Transform},
        Scene,
    },
    util::{self, ext::StrAsShaderDef},
    WgpuRenderer,
};
use glam::{UVec2, Vec2, Vec3};
use naga_oil::compose::ShaderDefValue;
use wgpu::{Surface, Texture, TextureFormat, TextureUsages, TextureViewDescriptor};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalSize, Size},
    event::{DeviceEvent, DeviceId, ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

use crate::{
    render::PbrRenderFlow,
    scene::{CameraConfig, ControllableCamera},
};

pub struct Application<'a> {
    renderer: WgpuRenderer,
    surface: Surface<'a>,
    window: Arc<Window>,
    depth_texture: Texture,
    dim: UVec2,

    main_camera: Arc<Mutex<ControllableCamera>>,
    scene: Scene,
    gpu_scene: GpuScene,
    shader_defs: HashMap<String, ShaderDefValue>,

    flow: PbrRenderFlow,
    last_draw: Instant,
    delta: f32,
}

impl<'a> Application<'a> {
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

        let renderer = WgpuRenderer::new().await;
        let surface = renderer.instance.create_surface(window.clone()).unwrap();
        surface.configure(
            &renderer.device,
            &surface
                .get_default_config(&renderer.adapter, dim.x, dim.y)
                .unwrap(),
        );

        let depth_texture = util::create_texture(
            &renderer.device,
            dim.extend(1),
            TextureFormat::Depth32Float,
            TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC,
        );

        let scene = crate::resource::load_primitives();

        let main_camera = ControllableCamera::new(
            Camera {
                transform: Transform {
                    translation: Vec3::new(0., 0., 0.),
                    ..Default::default()
                },
                projection: CameraProjection::Perspective(
                    aurora_core::scene::entity::PerspectiveProjection {
                        aspect_ratio: dim.x as f32 / dim.y as f32,
                        fov: std::f32::consts::FRAC_PI_4,
                        near: 0.1,
                        far: 1000.,
                    },
                ),
                // projection: CameraProjection::Orthographic(
                //     aurora_core::scene::entity::OrthographicProjection::symmetric(
                //         8., 4.5, -1000., 1000.,
                //     ),
                // ),
                exposure: Exposure { ev100: 9.7 },
            },
            CameraConfig::default(),
        );

        let shader_defs = [
            PbrSpecular::GGX.to_def(),
            PbrDiffuse::Lambert.to_def(),
            "TEX_NORMAL".to_def(),
        ]
        .into();

        let gpu_scene = GpuScene::default();

        Self {
            renderer,
            window,
            surface,
            depth_texture,
            dim,

            scene,
            gpu_scene,
            flow: Default::default(),
            shader_defs,

            main_camera: Arc::new(Mutex::new(main_camera)),

            last_draw: Instant::now(),
            delta: -1.,
        }
    }

    pub fn run(mut self, event_loop: EventLoop<()>) {
        let window = self.window.clone();
        let main_camera = self.main_camera.clone();
        let mut delta = 0.;

        thread::spawn(move || loop {
            let start = Instant::now();

            window.request_redraw();
            if let Ok(mut camera) = main_camera.lock() {
                camera.update(delta);
            }

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
        let screenshot = aurora_core::util::create_texture(
            &self.renderer.device,
            self.dim.extend(1),
            TextureFormat::Rgba8UnormSrgb,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        );

        let depth = aurora_core::util::create_texture(
            &self.renderer.device,
            self.dim.extend(1),
            TextureFormat::Depth32Float,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        );

        self.redraw(Some(RenderTargets {
            color_format: TextureFormat::Rgba8UnormSrgb,
            color: screenshot.create_view(&TextureViewDescriptor::default()),
            depth_format: Some(TextureFormat::Depth32Float),
            depth: Some(depth.create_view(&TextureViewDescriptor::default())),
        }));

        aurora_core::util::save_color_texture_as_image(
            "generated/screenshot.png",
            &screenshot,
            &self.renderer.device,
            &self.renderer.queue,
        )
        .await;
    }

    pub fn redraw(&mut self, target_override: Option<RenderTargets>) {
        let (Ok(frame), Ok(camera)) = (self.surface.get_current_texture(), self.main_camera.lock())
        else {
            return;
        };

        self.scene.camera = camera.camera;
        self.gpu_scene.sync(&mut self.scene, &self.renderer);

        let targets;
        if let Some(new_target) = target_override {
            self.flow.inner.build(
                &self.renderer,
                &self.scene,
                &mut self.gpu_scene,
                Some(self.shader_defs.clone()),
                &new_target,
            );

            targets = new_target;
        } else {
            let screen = RenderTargets {
                color_format: TextureFormat::Bgra8UnormSrgb,
                color: frame.texture.create_view(&TextureViewDescriptor::default()),
                depth_format: Some(TextureFormat::Depth32Float),
                depth: Some(
                    self.depth_texture
                        .create_view(&TextureViewDescriptor::default()),
                ),
            };

            targets = screen;
        }

        self.flow.inner.build(
            &self.renderer,
            &self.scene,
            &mut self.gpu_scene,
            Some(self.shader_defs.clone()),
            &targets,
        );

        self.flow
            .set_queue(self.scene.static_meshes.values().cloned().collect());

        self.flow
            .inner
            .run(&self.renderer, &self.scene, &mut self.gpu_scene, &targets);

        self.delta = self.last_draw.elapsed().as_secs_f32();
        self.last_draw = Instant::now();

        frame.present();
    }

    pub fn resize(&mut self, dim: UVec2) {
        self.dim = dim;
        self.surface.configure(
            &self.renderer.device,
            &self
                .surface
                .get_default_config(&self.renderer.adapter, dim.x, dim.y)
                .unwrap(),
        );
        self.depth_texture = util::create_texture(
            &self.renderer.device,
            dim.extend(1),
            TextureFormat::Depth32Float,
            TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC,
        );
    }
}

impl<'a> ApplicationHandler for Application<'a> {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                self.redraw(None);
            }
            WindowEvent::Resized(size) => self.resize(UVec2 {
                x: size.width,
                y: size.height,
            }),
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
