use aurora_core::render::scene::{
    ExtraBindGroupId, ExtraLayoutId, ExtraUniformBufferId, SamplerId, TextureId, TextureViewId,
};
use uuid::Uuid;

pub struct ShadowMapping {
    pub light_view_uniform: ExtraUniformBufferId,
    pub light_view_layout: ExtraLayoutId,
    pub light_view_bind_group: ExtraBindGroupId,
    pub directional_shadow_map: TextureId,
    pub directional_shadow_map_view: TextureViewId,
    pub point_shadow_map: TextureId,
    pub point_shadow_map_view: TextureViewId,
    pub shadow_map_sampler: SamplerId,

    pub shadow_maps_layout: ExtraLayoutId,
    pub shadow_maps_bind_group: ExtraBindGroupId,
}

pub const SHADOW_MAPPING: ShadowMapping = ShadowMapping {
    light_view_uniform: ExtraUniformBufferId(Uuid::from_u128(8794041105348641631856410231)),
    light_view_layout: ExtraLayoutId(Uuid::from_u128(7513015631563408941231)),
    light_view_bind_group: ExtraBindGroupId(Uuid::from_u128(123056463804784103210324847)),
    directional_shadow_map: TextureId(Uuid::from_u128(7861046541564897045132508964132)),
    directional_shadow_map_view: TextureViewId(Uuid::from_u128(10264856487964101541231456531)),
    point_shadow_map: TextureId(Uuid::from_u128(204153435154865423112313232)),
    point_shadow_map_view: TextureViewId(Uuid::from_u128(8974689406540351354897321563484)),
    shadow_map_sampler: SamplerId(Uuid::from_u128(8713416357854635486345415311523415)),
    shadow_maps_layout: ExtraLayoutId(Uuid::from_u128(9870130163543413521356876413)),
    shadow_maps_bind_group: ExtraBindGroupId(Uuid::from_u128(78974610032413605413136786)),
};
