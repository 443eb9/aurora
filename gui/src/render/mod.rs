use aurora_chest::node::{
    BasicTriangleNode, DepthPrepassNode, EnvironmentMappingNode, NormalPrepassNode, PbrNode,
    PbrNodeConfig, ShadowMappingNode, SsaoNode, ENVIRONMENT_MAP_PATH_ATTR,
};
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
        flow.add::<DepthPrepassNode>();
        flow.add::<NormalPrepassNode>();
        // flow.add::<PostProcessGeneralNode>();
        flow.add::<ShadowMappingNode>();
        flow.add_initialized(SsaoNode {
            denoise: true,
            ..Default::default()
        });
        // flow.add::<EnvironmentMappingNode>();
        flow.add_initialized(PbrNode {
            node_cfg: PbrNodeConfig::SSAO,
            ..Default::default()
        });
        // flow.add::<DepthViewNode>();

        // flow.config_node::<PbrNode>(PbrNodeConfig::ENVIRONMENT_MAPPING);
        // flow.add_extra_data::<EnvironmentMappingNode>(
        //     ENVIRONMENT_MAP_PATH_ATTR,
        //     "chest/assets/envmap/sunny_prairie_expanse_cube_map.hdr".into(),
        // );

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
