use std::{path::Path, time::Instant};

use glam::UVec2;

pub use log;
pub use wgpu::*;

use crate::{builtin_pipeline::AuroraPipeline, render::RenderTargets, scene::render::GpuScene};

pub mod buffer;
pub mod builtin_pipeline;
pub mod color;
pub mod render;
pub mod scene;
pub mod utils;

pub struct WgpuImageRenderer {
    internal: WgpuRenderer,
    target: Texture,
    target_view: TextureView,
    _depth_target: Texture,
    depth_target_view: TextureView,
}

impl WgpuImageRenderer {
    pub async fn new(dim: UVec2) -> Self {
        let renderer = WgpuRenderer::new().await;

        let (target, target_view) = utils::create_texture(
            &renderer.device,
            dim.extend(1),
            TextureFormat::Rgba8Unorm,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        );

        let (depth_target, depth_target_view) = utils::create_texture(
            &renderer.device,
            dim.extend(1),
            TextureFormat::Depth32Float,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        );

        Self {
            internal: renderer,
            target,
            target_view,
            _depth_target: depth_target,
            depth_target_view,
        }
    }

    #[inline]
    pub fn renderer(&self) -> &WgpuRenderer {
        &self.internal
    }

    #[inline]
    pub fn renderer_mut(&mut self) -> &mut WgpuRenderer {
        &mut self.internal
    }

    pub async fn draw<'a>(
        &'a self,
        scene: Option<&'a GpuScene>,
        pipeline: &'a impl AuroraPipeline<'a>,
    ) {
        self.internal.render(
            RenderTargets {
                color: &self.target_view,
                depth: Some(&self.depth_target_view),
            },
            scene,
            pipeline,
        );
    }

    pub async fn save_result(&self, path: impl AsRef<Path>) {
        utils::save_color_texture_as_image(
            path.as_ref().join("color.png"),
            &self.target,
            &self.internal.device,
            &self.internal.queue,
        )
        .await;

        // utils::save_depth_texture_as_image(
        //     path.as_ref().join("depth.png"),
        //     &self.depth_target,
        //     &self.internal.device,
        //     &self.internal.queue,
        // )
        // .await;
    }
}

pub struct WgpuSurfaceRenderer<'r> {
    internal: WgpuRenderer,
    surface: Surface<'r>,
    depth_target: Texture,
    depth_target_view: TextureView,
    last_printed_instant: Instant,
    frame_count: u32,
}

impl<'r> WgpuSurfaceRenderer<'r> {
    pub async fn new(target: impl Into<SurfaceTarget<'r>>, dim: UVec2) -> Self {
        let renderer = WgpuRenderer::new().await;
        let surface = renderer.instance.create_surface(target).unwrap();
        let (depth_target, depth_target_view) = utils::create_texture(
            &renderer.device,
            dim.extend(1),
            TextureFormat::Depth32Float,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        );

        let mut sr = Self {
            internal: renderer,
            surface,
            depth_target,
            depth_target_view,
            last_printed_instant: Instant::now(),
            frame_count: 0,
        };
        sr.resize(dim);
        sr
    }

    pub fn resize(&mut self, dim: UVec2) {
        (self.depth_target, self.depth_target_view) = utils::create_texture(
            &self.internal.device,
            dim.extend(1),
            TextureFormat::Depth32Float,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        );

        self.surface.configure(
            &self.internal.device,
            &SurfaceConfiguration {
                present_mode: PresentMode::AutoVsync,
                ..self
                    .surface
                    .get_default_config(&self.internal.adapter, dim.x, dim.y)
                    .unwrap()
            },
        );
    }

    pub fn draw<'a>(&'a self, scene: Option<&'a GpuScene>, pipeline: &'a impl AuroraPipeline<'a>) {
        let Ok(frame) = self.surface.get_current_texture() else {
            log::error!("Failed to acquire next swap chain texture.");
            return;
        };
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        self.internal.render(
            RenderTargets {
                color: unsafe { std::mem::transmute(&view) },
                depth: Some(&self.depth_target_view),
            },
            scene,
            pipeline,
        );
        frame.present();
    }

    pub fn update_frame_counter(&mut self) {
        self.frame_count += 1;
        let new_instant = Instant::now();
        let elapsed_secs = (new_instant - self.last_printed_instant).as_secs_f32();
        if elapsed_secs > 1.0 {
            let elapsed_ms = elapsed_secs * 1000.0;
            let frame_time = elapsed_ms / self.frame_count as f32;
            let fps = self.frame_count as f32 / elapsed_secs;
            log::info!("Frame time {:.2}ms ({:.1} FPS)", frame_time, fps);

            self.last_printed_instant = new_instant;
            self.frame_count = 0;
        }
    }

    #[inline]
    pub fn renderer(&self) -> &WgpuRenderer {
        &self.internal
    }

    #[inline]
    pub fn renderer_mut(&mut self) -> &mut WgpuRenderer {
        &mut self.internal
    }
}

pub struct WgpuRenderer {
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
}

impl WgpuRenderer {
    pub async fn new() -> Self {
        let instance = Instance::default();
        let adapter = instance
            .request_adapter(&RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    required_features: Features::empty(),
                    required_limits: Limits::downlevel_defaults(),
                },
                None,
            )
            .await
            .unwrap();

        log::info!("Wgpu context set up.");

        Self {
            instance,
            adapter,
            device,
            queue,
        }
    }

    pub fn render<'a>(
        &self,
        targets: RenderTargets<'a>,
        scene: Option<&'a GpuScene>,
        pipeline: &'a impl AuroraPipeline<'a>,
    ) {
        let mut command_encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });

        {
            let desc = pipeline.create_pass(targets);
            let mut pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: desc.label,
                color_attachments: &desc.color_attachments,
                depth_stencil_attachment: desc.depth_stencil_attachment,
                timestamp_writes: desc.timestamp_writes,
                occlusion_query_set: desc.occlusion_query_set,
            });

            pass.set_pipeline(pipeline.cache().expect("Pipeline is not built yet."));
            if let Some(bind_groups) = pipeline.bind(scene) {
                for (index, (bind_group, offsets)) in bind_groups.value.iter().enumerate() {
                    if let Some(offsets) = offsets {
                        pass.set_bind_group(index as u32, *bind_group, &offsets);
                    } else {
                        pass.set_bind_group(index as u32, *bind_group, &[]);
                    }
                }
            }
            pipeline.draw(&mut pass, scene);
        }

        self.queue.submit(Some(command_encoder.finish()));
    }

    #[inline]
    pub fn device(&self) -> &Device {
        &self.device
    }

    #[inline]
    pub fn queue(&self) -> &Queue {
        &self.queue
    }
}
