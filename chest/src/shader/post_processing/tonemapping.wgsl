#import aurora::fullscreen::FullscreenVertexOutput

@group(0) @binding(0) var color: texture_2d<f32>;
@group(0) @binding(1) var tony_mc_mapface_lut: texture_3d<f32>;
@group(0) @binding(2) var color_sampler: sampler;
@group(0) @binding(3) var lut_sampler: sampler;

fn tonemapping_reinhard(x: vec3f) -> vec3f {
    return x / (1. + x);
}

const TONY_MC_MAPFACE_LUT_DIMS: f32 = 48.0;

// Code from Bevy Engine
fn tonemapping_tony_mc_mapface(stimulus: vec3f) -> vec3f {
    var uv = (stimulus / (stimulus + 1.0)) * (f32(TONY_MC_MAPFACE_LUT_DIMS - 1.0) / f32(TONY_MC_MAPFACE_LUT_DIMS)) + 0.5 / f32(TONY_MC_MAPFACE_LUT_DIMS);
    return textureSampleLevel(tony_mc_mapface_lut, lut_sampler, uv, 0.0).rgb;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4f {
    let col = textureSample(color, color_sampler, in.uv).rgb;
#ifdef REINHARD
    let mapped = tonemapping_reinhard(col);
#else ifdef TONY_MC_MAPFACE
    let mapped = tonemapping_tony_mc_mapface(col);
#endif
    return vec4f(mapped, 1.0);
}
