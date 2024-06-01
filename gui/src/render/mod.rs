use aurora_chest::node::{BasicTriangleNode, PbrNode};
use aurora_core::render::flow::{GeneralNode, RenderFlow};

pub struct PbrRenderFlow {
    pub inner: RenderFlow,
}

impl Default for PbrRenderFlow {
    fn default() -> Self {
        let mut flow = RenderFlow::default();
        flow.add::<GeneralNode>();
        flow.add::<PbrNode>();
        Self { inner: flow }
    }
}

pub struct BasicTriangleRenderFlow {
    pub inner: RenderFlow,
}

impl Default for BasicTriangleRenderFlow {
    fn default() -> Self {
        let mut flow = RenderFlow::default();
        flow.add::<BasicTriangleNode>();
        Self { inner: flow }
    }
}
