#define_import_path aurora::env_mapping::env_mapping_binding
#import aurora::env_mapping::env_mapping_type::EnvironmentMapping

@group(#ENVIRONMENT_MAPPING) @binding(0) var env_map: texture_cube<f32>;
@group(#ENVIRONMENT_MAPPING) @binding(1) var irr_map: texture_cube<f32>;
@group(#ENVIRONMENT_MAPPING) @binding(2) var env_map_sampler: sampler;
@group(#ENVIRONMENT_MAPPING) @binding(3) var<uniform> env_mapping: EnvironmentMapping;
