use aurora_chest::node::PbrNode;
use aurora_core::render::flow::{CameraAndLightNode, RenderFlow};

pub struct PbrRenderFlow {
    pub inner: RenderFlow,
}

impl PbrRenderFlow {
    pub fn new() -> Self {
        let mut flow = RenderFlow::default();
        flow.add::<CameraAndLightNode>();
        flow.add::<PbrNode>();
        Self { inner: flow }
    }
}
