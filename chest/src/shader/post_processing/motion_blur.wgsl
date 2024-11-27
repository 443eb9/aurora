#import aurora::{
    fullscreen::FullscreenVertexOutput,
    math,
}

struct MotionVectorConfig {
    strength: f32,
    samples: u32,
    frame: u32,
}

@group(0) @binding(0) var color: texture_2d<f32>;
@group(0) @binding(1) var motion_vector: texture_2d<f32>;
@group(0) @binding(2) var color_sampler: sampler;
@group(0) @binding(3) var motion_vector_sampler: sampler;
@group(0) @binding(4) var<uniform> config: MotionVectorConfig;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4f {
    let motion = textureSample(motion_vector, motion_vector_sampler, in.uv).rg;

    let dim = vec2f(textureDimensions(color));
    let noise = math::interleaved_gradient_noise(dim * in.uv, config.frame);

    var col = textureSample(color, color_sampler, in.uv);
    for (var i = 0; i < i32(config.samples); i += 1) {
        let delta = (motion * (f32(i) + noise) * config.strength) / f32(config.samples);
        col += textureSample(color, color_sampler, in.uv - delta);
    }

    let weight = f32(config.samples + 1);
    return col / weight;
}
