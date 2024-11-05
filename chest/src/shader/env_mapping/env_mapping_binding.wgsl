#define_import_path aurora::env_mapping::env_mapping_binding
#import aurora::env_mapping::env_mapping_type::EnvironmentMapping

@group(4) @binding(0) var refl_map: texture_cube<f32>;
@group(4) @binding(1) var irradiance_map: texture_cube<f32>;
@group(4) @binding(2) var env_map_sampler: sampler;
@group(4) @binding(3) var<uniform> env_mapping: EnvironmentMapping;
