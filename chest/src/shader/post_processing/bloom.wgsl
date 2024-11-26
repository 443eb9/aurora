#import aurora::{
    fullscreen::FullscreenVertexOutput,
    math,
}

struct BloomConfig {
    precomputed_filter: vec4f,
}

@group(0) @binding(0) var color: texture_2d<f32>;
@group(0) @binding(1) var color_sampler: sampler;
@group(0) @binding(2) var<uniform> bloom_config: BloomConfig;

fn soft_threshold(c: vec3f) -> vec3f {
    let f = bloom_config.precomputed_filter;
    let brightness = max(c.x, max(c.y, c.z));
    var soft = brightness - f.y;
    soft = clamp(soft, 0.0, f.z);
    soft = soft * soft * f.w;
    var contribution = max(soft, brightness - f.x);
    contribution /= max(brightness, 0.000001);
    return c * contribution;
}

fn luminance(c: vec3f) -> f32 {
    return c.r * 0.2126 + c.g * 0.7152 + c.b * 0.0722;
}

fn karis_average(c: vec3f) -> f32 {
    let luma = math::luminance(math::linear_to_srgb(c)) * 0.25;
    return 1.0 / (1.0 + luma);
}

@fragment
fn downsample(in: FullscreenVertexOutput) -> @location(0) vec4f {
    let uv = in.uv;
    // a - b - c
    // - j - k -
    // d - e - f
    // - l - m -
    // g - h - i

    let a = textureSample(color, color_sampler, uv, vec2i(-2, -2)).rgb;
    let b = textureSample(color, color_sampler, uv, vec2i( 0, -2)).rgb;
    let c = textureSample(color, color_sampler, uv, vec2i( 2, -2)).rgb;

    let d = textureSample(color, color_sampler, uv, vec2i(-2,  0)).rgb;
    let e = textureSample(color, color_sampler, uv).rgb;
    let f = textureSample(color, color_sampler, uv, vec2i( 2,  0)).rgb;

    let g = textureSample(color, color_sampler, uv, vec2i(-2,  2)).rgb;
    let h = textureSample(color, color_sampler, uv, vec2i( 0,  2)).rgb;
    let i = textureSample(color, color_sampler, uv, vec2i( 2,  2)).rgb;

    let j = textureSample(color, color_sampler, uv, vec2i(-1, -1)).rgb;
    let k = textureSample(color, color_sampler, uv, vec2i( 1, -1)).rgb;
    let l = textureSample(color, color_sampler, uv, vec2i(-1,  1)).rgb;
    let m = textureSample(color, color_sampler, uv, vec2i( 1,  1)).rgb;

#ifdef FIRST_DOWNSAMPLE
    var group0 = (a + b + d + e) * 0.03125;
    var group1 = (b + c + e + f) * 0.03125;
    var group2 = (d + e + g + h) * 0.03125;
    var group3 = (e + f + h + i) * 0.03125;
    var group4 = (j + k + l + m) * 0.125;
    group0 *= karis_average(group0);
    group1 *= karis_average(group1);
    group2 *= karis_average(group2);
    group3 *= karis_average(group3);
    group4 *= karis_average(group4);
    var col = group0 + group1 + group2 + group3 + group4;
#ifdef SOFT_THRESHOLD
    col = soft_threshold(col);
#endif // SOFT_THRESHOLD

#else // FIRST_DOWNSAMPLE
    let col = e * 0.125 + (a + c + g + i) * 0.03125 + (b + d + f + h) * 0.0625 + (j + k + l + m) * 0.125;
#endif // FIRST_DOWNSAMPLE
    return vec4f(col, 1.0);
}

@fragment
fn upsample(in: FullscreenVertexOutput) -> @location(0) vec4f {
    let uv = in.uv;
    // a - b - c
    // d - e - f
    // g - h - i
    let a = textureSample(color, color_sampler, uv, vec2i(-1, -1)).rgb;
    let b = textureSample(color, color_sampler, uv, vec2i( 0, -1)).rgb;
    let c = textureSample(color, color_sampler, uv, vec2i( 1, -1)).rgb;

    let d = textureSample(color, color_sampler, uv, vec2i(-1,  0)).rgb;
    let e = textureSample(color, color_sampler, uv).rgb;
    let f = textureSample(color, color_sampler, uv, vec2i( 1,  0)).rgb;
    
    let g = textureSample(color, color_sampler, uv, vec2i(-1,  1)).rgb;
    let h = textureSample(color, color_sampler, uv, vec2i( 0,  1)).rgb;
    let i = textureSample(color, color_sampler, uv, vec2i( 1,  1)).rgb;

    let col = e * 0.25 + (b + d + f + h) * 0.125 + (a + c + g + i) * 0.0625;
    return vec4f(col, 1.0);
}
