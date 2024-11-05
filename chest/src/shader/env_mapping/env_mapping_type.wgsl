#define_import_path aurora::env_mapping::env_mapping_type

struct EnvironmentMapping {
    intensity: f32,
}

struct EnvironmentMapConvolution {
    elevation_samples: u32,
    azimuth_samples: u32,
    sample_distance: f32,
}
