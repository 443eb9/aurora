use aurora_chest::node::{BasicTriangleNode, PbrNode};
use aurora_core::render::flow::{GeneralNode, ImageFallbackNode, RenderFlow};
use uuid::Uuid;

pub struct PbrRenderFlow {
    pub inner: RenderFlow,
    pub ids: Vec<Uuid>,
}

impl Default for PbrRenderFlow {
    fn default() -> Self {
        let mut ids = Vec::new();
        let mut flow = RenderFlow::default();
        ids.push(flow.add::<GeneralNode>());
        ids.push(flow.add::<ImageFallbackNode>());
        ids.push(flow.add::<PbrNode>());
        // ids.push(flow.add::<DepthViewNode>());

        Self { inner: flow, ids }
    }
}

pub struct BasicTriangleRenderFlow {
    pub inner: RenderFlow,
    pub ids: Vec<Uuid>,
}

impl Default for BasicTriangleRenderFlow {
    fn default() -> Self {
        let mut ids = Vec::new();
        let mut flow = RenderFlow::default();
        ids.push(flow.add::<BasicTriangleNode>());
        Self { inner: flow, ids }
    }
}
