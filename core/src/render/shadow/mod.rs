use std::collections::HashMap;

use naga_oil::compose::ShaderDefValue;
use uuid::Uuid;

use crate::{
    render::{
        flow::RenderNode,
        resource::{RenderMesh, RenderTargets},
        scene::GpuScene,
    },
    WgpuRenderer,
};

pub const SHADOW_MAP_UUID: Uuid = Uuid::from_u128(9412060231169401541323186749641);

#[derive(Clone)]
pub struct ShadowMapNode {
    
}

impl RenderNode for ShadowMapNode {
    fn build(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        shader_defs: Option<HashMap<String, ShaderDefValue>>,
        target: &RenderTargets,
    ) {
        todo!()
    }

    fn prepare(
        &mut self,
        renderer: &WgpuRenderer,
        scene: &mut GpuScene,
        queue: &mut [RenderMesh],
        target: &RenderTargets,
    ) {
        todo!()
    }

    fn draw(
        &self,
        renderer: &WgpuRenderer,
        scene: &GpuScene,
        queue: &[RenderMesh],
        target: &RenderTargets,
    ) {
        todo!()
    }
}
