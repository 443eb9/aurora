use std::borrow::Cow;

use aurora_core::render::{
    flow::{RenderContext, RenderNode},
    scene::GpuScene,
};
use wgpu::{
    Color, ColorTargetState, ColorWrites, CommandEncoderDescriptor, FragmentState, LoadOp,
    MultisampleState, Operations, PipelineCompilationOptions, PrimitiveState,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    ShaderModuleDescriptor, ShaderSource, StoreOp, VertexState,
};

#[derive(Default)]
pub struct BasicTriangleNode {
    pipeline: Option<RenderPipeline>,
}

impl RenderNode for BasicTriangleNode {
    fn build(
        &mut self,
        _scene: &mut GpuScene,
        RenderContext {
            device, targets, ..
        }: RenderContext,
    ) {
        let shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("basic_triangle_shader"),
            source: ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "../shader/basic_triangle.wgsl"
            ))),
        });

        self.pipeline = Some(device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("basic_triagle_pipeline"),
            layout: None,
            cache: None,
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vertex",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fragment",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: targets.color_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        }));
    }

    fn draw(
        &self,
        _scene: &mut GpuScene,
        RenderContext {
            device,
            queue,
            targets,
            ..
        }: RenderContext,
    ) {
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("basic_triangle_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &targets.color,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::GREEN),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            pass.set_pipeline(self.pipeline.as_ref().unwrap());
            pass.draw(0..3, 0..1);
        }

        queue.submit(Some(encoder.finish()));
    }
}
