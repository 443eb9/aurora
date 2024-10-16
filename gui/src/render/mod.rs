use aurora_chest::node::{BasicTriangleNode, DepthViewNode, PbrNode, ShadowMappingNode};
use aurora_core::render::{
    flow::{GeneralNode, ImageFallbackNode, PostProcessGeneralNode, RenderFlow},
    mesh::StaticMesh,
};
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
        ids.push(flow.add::<PostProcessGeneralNode>());
        ids.push(flow.add::<ShadowMappingNode>());
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
