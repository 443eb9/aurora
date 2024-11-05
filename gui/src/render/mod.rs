use aurora_chest::node::{
    BasicTriangleNode, EnvironmentMappingNode, PbrNode, PbrNodeConfig, ShadowMappingNode,
    ENVIRONMENT_MAP_PATH_ATTR,
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
        flow.add::<PostProcessGeneralNode>();
        flow.add::<ShadowMappingNode>();
        flow.add::<EnvironmentMappingNode>();
        flow.add::<PbrNode>();
        // flow.add::<DepthViewNode>();

        flow.config_node::<PbrNode>(PbrNodeConfig::SHADOW_MAPPING);
        flow.config_node::<PbrNode>(PbrNodeConfig::ENVIRONMENT_MAPPING);
        flow.add_extra_data::<EnvironmentMappingNode>(
            ENVIRONMENT_MAP_PATH_ATTR,
            "chest/assets/envmap/german_town_corner_at_night_cube_map.hdr".into(),
        );

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
