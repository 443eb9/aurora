use aurora_chest::node::{BasicTriangleNode, PbrNode, PbrNodeConfig, ShadowMappingNode};
use aurora_core::render::flow::{
    GeneralNode, ImageFallbackNode, PostProcessGeneralNode, RenderFlow,
};

pub struct PbrRenderFlow {
    pub inner: RenderFlow,
}

impl Default for PbrRenderFlow {
    fn default() -> Self {
        let mut flow = RenderFlow::default();
        flow.add::<GeneralNode>();
        flow.add::<ImageFallbackNode>();
        flow.add::<PostProcessGeneralNode>();
        flow.add::<ShadowMappingNode>();
        flow.add::<PbrNode>();
        // flow.add::<DepthViewNode>();

        flow.config_node::<PbrNode>(PbrNodeConfig::SHADOW_MAPPING);

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
