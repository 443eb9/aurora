use std::f32::consts::FRAC_PI_4;

use app::Application;

use aurora_core::{
    color::SrgbaColor,
    node::{AuroraRenderFlow, DepthPassNode, PbrNode},
    scene::{
        component::{CameraProjection, Mesh, PerspectiveProjection, Transform},
        entity::{Camera, DirectionalLight, Light},
        render::GpuScene,
        Scene,
    },
    wgpu::TextureFormat,
    WgpuImageRenderer,
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
        renderer.device(),
    );
    gpu_scene.write_scene(renderer.device(), renderer.queue());

    // renderer.save_result("generated/color.png").await;
    let pbr_node = PbrNode::new(TextureFormat::Rgba8Unorm);
    let depth_pass_node = DepthPassNode::new(TextureFormat::Rgba8Unorm);
    let mut flow = AuroraRenderFlow::default();
    flow.add("pbr".into(), Box::new(pbr_node));
    flow.add("depth_pass".into(), Box::new(depth_pass_node));

    flow.build(renderer.device(), None);
    flow.prepare(renderer.device(), &renderer.targets(), Some(&gpu_scene));
    renderer.draw(Some(&gpu_scene), &flow).await;
    renderer.save_result("generated/depth.png").await;
}

async fn realtime_render(dim: UVec2) {
    let event_loop = EventLoop::new().unwrap();
    let app = Application::new(&event_loop, dim).await;
    app.run(event_loop);
}

fn main() {
    env_logger::builder()
        .filter_level(aurora_core::log::LevelFilter::Info)
        .init();

    // pollster::block_on(render_to_image(WINDOW_DIM));
    pollster::block_on(realtime_render(WINDOW_DIM));
}
