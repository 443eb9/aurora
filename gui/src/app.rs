use std::{
    any::Any,
    f32::consts::FRAC_PI_4,
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};

use aurora_chest::material::PbrMaterial;
use aurora_core::{
    render::{flow::RenderFlow, resource::RenderTarget, scene::GpuScene},
    scene::{
        entity::{
            Camera, CameraProjection, DirectionalLight, Light, PerspectiveProjection, StaticMesh,
            Transform,
        },
        resource::Mesh,
        Scene,
    },
    util::{self, TypeIdAsUuid},
    WgpuRenderer,
};
use glam::{EulerRot, Mat3, Mat4, Quat, UVec2, Vec2, Vec3};

use palette::Srgb;
use uuid::Uuid;
use wgpu::{Surface, SurfaceConfiguration, TextureFormat, TextureUsages, TextureViewDescriptor};
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
    targets: RenderTarget,
    dim: UVec2,

    main_camera: Arc<Mutex<ControllableCamera>>,
    scene: Scene,
    gpu_scene: GpuScene,

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
        let targets = RenderTarget {
            color: surface
                .get_current_texture()
                .unwrap()
                .texture
                .create_view(&TextureViewDescriptor::default()),
            depth: Some(
                util::create_texture(
                    &renderer.device,
                    dim.extend(1),
                    TextureFormat::Depth32Float,
                    TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
                )
                .create_view(&TextureViewDescriptor::default()),
            ),
        };

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

        let pbr_material = PbrMaterial {
            base_color: Srgb::new(1., 1., 1.),
            tex_base_color: None,
            metallic: 0.,
            roughness: 1.,
        };

        let mut scene = Scene::default();
        let material_uuid = scene.insert_object(pbr_material);
        let meshes = Mesh::from_obj("assets/large_model_sphere.obj")
            .into_iter()
            .map(|m| scene.insert_object(m))
            .collect::<Vec<_>>();
        let static_meshes = meshes
            .into_iter()
            .map(|mesh| StaticMesh {
                mesh,
                material: material_uuid,
            })
            .collect::<Vec<_>>();

        scene.lights.push(Light::Directional(DirectionalLight {
            transform: Transform {
                rotation: Quat::from_mat4(&Mat4::look_to_rh(
                    Vec3::ZERO,
                    Vec3::new(0., 1., 0.),
                    Vec3::Y,
                )),
                ..Default::default()
            },
            ..Default::default()
        }));
        static_meshes.into_iter().for_each(|sm| {
            scene.insert_object(sm);
        });

        let gpu_scene = GpuScene::default();

        Self {
            renderer,
            window,
            surface,
            targets,
            dim,

            scene,
            gpu_scene,
            flow: PbrRenderFlow::new(),

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
        // let mut flow = RenderFlow::default();
        // flow.add(
        //     "pbr".into(),
        //     Box::new(PbrNode::new(TextureFormat::Rgba8Unorm)),
        // );
        // flow.add(
        //     "depth_pass".into(),
        //     Box::new(DepthPassNode::new(TextureFormat::Rgba8Unorm)),
        // );

        // flow.build(self.renderer.device(), None);
        // flow.prepare(
        //     self.renderer.device(),
        //     &self.renderer.targets(),
        //     Some(&self.gpu_scene),
        // );

        // let (screenshot, screenshot_view) = aurora_core::utils::create_texture(
        //     self.renderer.device(),
        //     self.dim.extend(1),
        //     TextureFormat::Rgba8Unorm,
        //     TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        // );

        // let (_depth, depth_view) = aurora_core::utils::create_texture(
        //     self.renderer.device(),
        //     self.dim.extend(1),
        //     TextureFormat::Depth32Float,
        //     TextureUsages::RENDER_ATTACHMENT,
        // );

        // self.renderer.renderer().render(
        //     &RenderTargets {
        //         color: &screenshot_view,
        //         depth: Some(&depth_view),
        //     },
        //     Some(&self.gpu_scene),
        //     &flow,
        // );

        // aurora_core::utils::save_color_texture_as_image(
        //     "generated/screenshot.png",
        //     &screenshot,
        //     self.renderer.device(),
        //     self.renderer.queue(),
        // )
        // .await;
    }

    pub fn redraw(&mut self) {
        self.gpu_scene.sync(&mut self.scene, &self.renderer);
        self.flow.inner.build(&self.renderer, &self.gpu_scene, None);
        self.flow.inner.queue = self.scene.static_meshes.values().cloned().collect();
        self.flow
            .inner
            .run(&self.renderer, &mut self.gpu_scene, &self.targets);
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
            WindowEvent::RedrawRequested => self.redraw(),
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
