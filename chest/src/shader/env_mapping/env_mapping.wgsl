#define_import_path aurora::env_mapping::env_mapping
#import aurora::env_mapping::env_mapping_binding::{refl_map, irradiance_map, env_map_sampler, env_mapping}

fn sample_refl_map(dir: vec3f) -> vec3f {
    return textureSample(refl_map, env_map_sampler, dir).rgb * env_mapping.intensity;
}

fn sample_irradiance_map(normal: vec3f) -> vec3f {
    return textureSample(irradiance_map, env_map_sampler, normal).rgb * env_mapping.intensity;
}
