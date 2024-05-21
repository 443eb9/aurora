use std::{borrow::Cow, f32::consts::FRAC_PI_4};

use app::Application;

use aurora_core::{
    builtin_pipeline::{AuroraPipeline, DepthPassPipeline, PbrPipeline},
    color::SrgbaColor,
    scene::{
        component::{CameraProjection, Mesh, PerspectiveProjection, Transform},
        entity::{Camera, DirectionalLight, Light},
        render::GpuScene,
        Scene,
    },
    TextureFormat, WgpuImageRenderer,
};

use glam::{EulerRot, Quat, UVec2, Vec3};

use winit::event_loop::EventLoop;

mod app;
mod scene;

const WINDOW_DIM: UVec2 = UVec2::new(1920, 1080);

async fn render_to_image(dim: UVec2) {
    let renderer = WgpuImageRenderer::new(dim).await;

    let mut scene = Scene {
        camera: Camera {
            transform: Transform {
                translation: Vec3::new(0., 0., -5.),
                ..Default::default()
            },
            projection: CameraProjection::Perspective(PerspectiveProjection {
                aspect_ratio: dim.x as f32 / dim.y as f32,
                fov: FRAC_PI_4,
                near: 0.1,
                far: 1000.,
            }),
        },
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

    let mut pbr_pipeline = PbrPipeline::new(device, TextureFormat::Rgba8Unorm);
    pbr_pipeline.build(device, Default::default());
    renderer.draw(Some(&gpu_scene), &mut pbr_pipeline).await;
    renderer.save_result("generated/color.png").await;

    let mut depth_pass_pipeline = DepthPassPipeline::new(device, TextureFormat::Rgba8Unorm);
    renderer.draw(None, &mut depth_pass_pipeline).await;
    renderer.save_result("generated/depth.png").await;
}

async fn realtime_render(dim: UVec2) {
    let event_loop = EventLoop::new().unwrap();
    let mut app = Application::new(&event_loop, dim).await;
    app.run(event_loop);
}

fn main() {
    env_logger::builder()
        .filter_level(aurora_core::log::LevelFilter::Info)
        .init();

    pollster::block_on(render_to_image(WINDOW_DIM));
    // pollster::block_on(realtime_render(WINDOW_DIM));
}
