use std::{borrow::Cow, f32::consts::FRAC_PI_4};

use app::Application;

use aurora_core::{
    color::SrgbaColor,
    render::Vertex,
    scene::{
        component::{CameraProjection, Mesh, PerspectiveProjection, Transform},
        entity::{Camera, DirectionalLight, Light},
        render::GpuScene,
        Scene,
    },
    vertex_attr_array, BufferAddress, CompareFunction, DepthBiasState, DepthStencilState,
    FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, RenderPipelineDescriptor, ShaderModuleDescriptor, ShaderSource, StencilState,
    TextureFormat, VertexBufferLayout, VertexState, VertexStepMode, WgpuImageRenderer,
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
    let shader_module = device.create_shader_module(ShaderModuleDescriptor {
        label: None,
        source: ShaderSource::Wgsl(Cow::Borrowed(include_str!("../assets/shader.wgsl"))),
    });
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&gpu_scene.b_camera.layout, &gpu_scene.b_lights.layout],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader_module,
            entry_point: "vertex",
            compilation_options: PipelineCompilationOptions::default(),
            buffers: &[VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
                step_mode: VertexStepMode::Vertex,
                attributes: &vertex_attr_array![0 => Float32x3, 1 => Float32x3],
            }],
        },
        fragment: Some(FragmentState {
            module: &shader_module,
            entry_point: "fragment",
            compilation_options: PipelineCompilationOptions::default(),
            targets: &[Some(TextureFormat::Rgba8Unorm.into())],
        }),
        primitive: PrimitiveState::default(),
        depth_stencil: Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::LessEqual,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        }),
        multisample: MultisampleState::default(),
        multiview: None,
    });

    renderer.draw(&gpu_scene, &pipeline).await;
    renderer.save_result("generated/").await;
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
