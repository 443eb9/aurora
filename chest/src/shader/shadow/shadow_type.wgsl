#define_import_path aurora::shadow_type

struct ShadowMappingConfig {
    dir_map_resolution: u32,
    point_map_resolution: u32,
    samples: u32,
    dir_pcf_radius: f32,
    dir_pcss_radius: f32,
    point_pcf_radius: f32,
    point_pcss_radius: f32,
}
