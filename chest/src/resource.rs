use std::sync::Mutex;

use once_cell::sync::Lazy;
use wgpu::{Sampler, Texture};

pub struct ShadowMaps {
    pub directional_shadow_map: Texture,
    pub directional_shadow_sampler: Sampler,
    pub point_shadow_map: Texture,
    pub point_shadow_sampler: Sampler,
}

pub static SHADOW_MAPS: Lazy<Mutex<Option<ShadowMaps>>> = Lazy::new(|| Mutex::default());
