#define_import_path aurora::tonemapping

@group(2) @binding(#LUT_TEX_BINDING) var tony_mc_mapface_lut: texture_3d<f32>;
@group(2) @binding(#LUT_SAMPLER_BINDING) var tony_mc_mapface_lut_sampler: sampler;

fn tonemapping_reinhard(x: vec3f) -> vec3f {
    return x / (1. + x);
}

const TONY_MC_MAPFACE_LUT_DIMS: f32 = 48.0;

// Code from Bevy Engine
fn tonemapping_tony_mc_mapface(stimulus: vec3f) -> vec3f {
    var uv = (stimulus / (stimulus + 1.0)) * (f32(TONY_MC_MAPFACE_LUT_DIMS - 1.0) / f32(TONY_MC_MAPFACE_LUT_DIMS)) + 0.5 / f32(TONY_MC_MAPFACE_LUT_DIMS);
    return textureSampleLevel(tony_mc_mapface_lut, tony_mc_mapface_lut_sampler, uv, 0.0).rgb;
}
