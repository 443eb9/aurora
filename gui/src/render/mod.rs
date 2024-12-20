use aurora_chest::node::{
    BasicTriangleNode, BloomNode, DepthOfFieldNode, DepthPrepassNode, EnvironmentMappingNode,
    EnvironmentMappingNodeConfig, LensFlareNode, MotionBlurNode, MotionVectorPrepassNode,
    NormalPrepassNode, PbrNode, PbrNodeConfig, ShadowMappingNode, ShadowMappingNodeConfig,
    SkyboxNode, SkyboxNodeConfig, SsaoNode, TonemappingNode, ENVIRONMENT_MAP_PATH_ATTR,
};
use aurora_core::render::flow::{
    GeneralNode, ImageFallbackNode, PostProcessGeneralNode, PresentNode, RenderFlow,
};

pub struct PbrRenderFlow {
    pub inner: RenderFlow,
}

impl Default for PbrRenderFlow {
    fn default() -> Self {
        let mut flow = RenderFlow::default();
        flow.add::<GeneralNode>()
            .add::<ImageFallbackNode>()
            .add::<DepthPrepassNode>()
            .add::<NormalPrepassNode>()
            .add::<MotionVectorPrepassNode>()
            // .add_initialized(ShadowMappingNode {
            //     node_cfg: ShadowMappingNodeConfig::RANDOMIZE,
            //     ..Default::default()
            // })
            // .add_initialized(SsaoNode {
            //     denoise: true,
            //     debug_ssao_only: false,
            //     ..Default::default()
            // })
            // .add_initialized(PbrNode {
            //     node_cfg: PbrNodeConfig::SSAO | PbrNodeConfig::SHADOW_MAPPING,
            //     ..Default::default()
            // })
            .add_initialized(EnvironmentMappingNode {
                node_config: EnvironmentMappingNodeConfig {
                    env_map_path: "chest/assets/envmap/sunny_prairie_expanse_cube_map.hdr".into(),
                },
                data: None,
                config: Default::default(),
                convolution_config: Default::default(),
            })
            // .add_initialized(SkyboxNode {
            //     node_config: SkyboxNodeConfig {
            //         skybox_path: "chest/assets/envmap/sunny_prairie_expanse_cube_map.hdr".into(),
            //     },
            //     data: None,
            // })
            // .add::<PbrNode>()
            .add_initialized(PbrNode {
                node_cfg: PbrNodeConfig::ENVIRONMENT_MAPPING,
                ..Default::default()
            })
            // .add::<BloomNode>()
            // .add::<DepthOfFieldNode>()
            // .add::<LensFlareNode>()
            // .add::<MotionBlurNode>()
            // .add::<PresentNode>()
            .add::<TonemappingNode>();

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
