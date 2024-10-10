use std::sync::{Arc, Mutex};

use aurora_core::WgpuRenderer;
use once_cell::sync::Lazy;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindingResource, Sampler,
    Texture, TextureView,
};

pub struct ShadowMaps {
    pub directional_shadow_map: Texture,
    pub directional_shadow_map_view: TextureView,
    pub point_shadow_map: Texture,
    pub point_shadow_map_view: TextureView,
    pub shadow_map_sampler: Sampler,

    pub layout: Option<Arc<BindGroupLayout>>,
}

pub static SHADOW_MAPS: Lazy<Mutex<Option<ShadowMaps>>> = Lazy::new(|| Mutex::default());

impl ShadowMaps {
    pub fn create_binding(
        &self,
        renderer: &WgpuRenderer,
        light_views: BindingResource,
    ) -> Option<BindGroup> {
        let Some(shadow_layout) = &self.layout else {
            return None;
        };

        Some(renderer.device.create_bind_group(&BindGroupDescriptor {
            label: Some("shadow_binding"),
            layout: &shadow_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: light_views,
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.shadow_map_sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&self.directional_shadow_map_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&self.point_shadow_map_view),
                },
                // BindGroupEntry {
                //     binding: 4,
                //     resource: BindingResource::TextureView(&self.spot_shadow_map_view),
                // },
            ],
        }))
    }
}
