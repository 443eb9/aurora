#import aurora::{common_binding::camera, fullscreen::FullscreenVertexOutput, math}

struct DofConfig {
    focal_length: f32,
    focal_distance: f32,
    coc_factor: f32,
    max_coc_radius: f32,
    max_depth: f32,
}

@group(1) @binding(0) var depth: texture_depth_2d;
@group(1) @binding(1) var color: texture_2d<f32>;
@group(1) @binding(2) var color_sampler: sampler;
@group(1) @binding(3) var<uniform> config: DofConfig;

fn calculate_coc_radius(uv: vec2f) -> f32 {
    let dim = vec2f(textureDimensions(depth));
    let clip_z = textureLoad(depth, vec2i(uv * dim), 0);
    let z = min(config.max_depth, math::clip_depth_to_view(clip_z, camera.inv_proj));

    let d = config.coc_factor * abs(z - config.focal_distance) / (z * (config.focal_distance - config.focal_length));
    return min(config.max_coc_radius, d * dim.y * 0.5);
}

fn gaussian_blur(uv: vec2f, coc_radius: f32, step_texel_offset: vec2f) -> vec4f {
    let sigma = coc_radius * 0.5;
    let samples = i32(ceil(sigma * 1.5));
    let step_uv_offset = step_texel_offset / vec2f(textureDimensions(color));
    let exp_factor = -1.0 / (2.0 * sigma * sigma);

    var sum = textureSample(color, color_sampler, uv).rgb;
    var weight_sum = 1.0;

    // for (var step = 1; step <= samples; step += 2) {
    //     let w0 = exp(exp_factor * f32(step) * f32(step));
    //     let w1 = exp(exp_factor * f32(step + 1) * f32(step + 1));
    //     let uv_offset = step_uv_offset * (f32(step) + w1 / (w0 + w1));
    //     let weight = w0 + w1;

    //     sum += (
    //         textureSampleLevel(color, color_sampler, uv + uv_offset, 0.0).rgb +
    //         textureSampleLevel(color, color_sampler, uv - uv_offset, 0.0).rgb
    //     ) * weight;
    //     weight_sum += weight * 2.0;
    // }

    for (var step = 1; step <= samples; step += 2) {
        let weight = exp(exp_factor * f32(step) * f32(step));
        let uv_offset = step_uv_offset * f32(step);

        sum += (
            textureSampleLevel(color, color_sampler, uv + uv_offset, 0.0).rgb +
            textureSampleLevel(color, color_sampler, uv - uv_offset, 0.0).rgb
        ) * weight;
        weight_sum += weight * 2.0;
    }

    return vec4f(sum / weight_sum, 1.0);
}

@fragment
fn gaussian_horizontal(in: FullscreenVertexOutput) -> @location(0) vec4f {
    let coc = calculate_coc_radius(in.uv);
    return gaussian_blur(in.uv, coc, vec2f(1.0, 0.0));
}

@fragment
fn gaussian_vertical(in: FullscreenVertexOutput) -> @location(0) vec4f {
    let coc = calculate_coc_radius(in.uv);
    return gaussian_blur(in.uv, coc, vec2f(0.0, 1.0));
}
