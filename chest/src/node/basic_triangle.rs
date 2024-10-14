use std::{borrow::Cow, collections::HashMap};

use aurora_core::{
    render::{
        flow::RenderNode,
        resource::{RenderMesh, RenderTargets},
        scene::GpuScene,
    },
    WgpuRenderer,
};
use naga_oil::compose::ShaderDefValue;
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
        renderer: &WgpuRenderer,
        _scene: &mut GpuScene,
        _shader_defs: Option<HashMap<String, ShaderDefValue>>,
        target: &RenderTargets,
    ) {
        let shader_module = renderer
            .device
            .create_shader_module(ShaderModuleDescriptor {
                label: Some("basic_triangle_shader"),
                source: ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                    "../shader/basic_triangle.wgsl"
                ))),
            });

        self.pipeline = Some(
            renderer
                .device
                .create_render_pipeline(&RenderPipelineDescriptor {
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
                            format: target.color_format,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    primitive: PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    multiview: None,
                }),
        );
    }

    fn prepare(
        &mut self,
        _renderer: &WgpuRenderer,
        _scene: &mut GpuScene,
        _queue: &mut [RenderMesh],
        _target: &RenderTargets,
    ) {
    }

    fn draw(
        &self,
        renderer: &WgpuRenderer,
        _scene: &mut GpuScene,
        _queue: &[RenderMesh],
        target: &RenderTargets,
    ) {
        let mut encoder = renderer
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("basic_triangle_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &target.color,
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

        renderer.queue.submit(Some(encoder.finish()));
    }
}
