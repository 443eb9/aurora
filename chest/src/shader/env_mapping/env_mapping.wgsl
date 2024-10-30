#define_import_path aurora::env_mapping::env_mapping
#import aurora::env_mapping::env_mapping_binding::{env_map, env_map_sampler, env_mapping}

fn sample_env_map(dir: vec3f) -> vec3f {
    return textureSample(env_map, env_map_sampler, dir).rgb * env_mapping.intensity;
}
