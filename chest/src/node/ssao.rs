use aurora_core::render::{
    flow::{RenderContext, RenderNode},
    scene::GpuScene,
};
use wgpu::{RenderPipeline, RenderPipelineDescriptor};

#[derive(Default)]
pub struct SsaoNode {}

impl RenderNode for SsaoNode {
    fn build(
        &mut self,
        scene: &mut GpuScene,
        RenderContext {
            device,
            queue,
            node,
            targets,
        }: RenderContext,
    ) {
    }
}
