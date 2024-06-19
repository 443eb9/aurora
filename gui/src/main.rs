use glam::UVec2;
use winit::event_loop::EventLoop;

mod app;
mod render;
mod resource;
mod scene;

async fn real_time_app() {
    let event_loop = EventLoop::new().unwrap();

    let app = app::Application::new(&event_loop, UVec2::new(1920, 1080)).await;
    app.run(event_loop);
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    pollster::block_on(real_time_app());
}
