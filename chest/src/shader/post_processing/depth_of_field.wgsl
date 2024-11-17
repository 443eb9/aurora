#import aurora::{common_binding::camera, fullscreen::FullscreenVertexOutput, math}

struct DofConfig {
    aperture_diameter: f32,
    focal_length: f32,
    coc_factor: f32,
    max_coc_radius: f32,
}

@group(1) @binding(0) var depth: texture_depth_2d;
@group(1) @binding(1) var color: texture_2d<f32>;
@group(1) @binding(2) var color_sampler: sampler;
@group(1) @binding(3) var<uniform> config: DofConfig;

fn calculate_coc_radius(uv: vec2f) -> f32 {
    let z = math::clip_depth_to_view(textureLoad(depth, vec2i(uv * vec2f(textureDimensions(depth))), 0), camera.inv_proj);
    let d = config.aperture_diameter * abs(z - config.focal_length) / config.focal_length * config.coc_factor;
    return min(config.max_coc_radius, d * 0.5);
}

fn gaussian(uv: vec2f, coc_radius: f32, step_texel_offset: vec2f) -> vec4f {
    let sigma = coc_radius;
    let half_samples = i32(ceil(sigma * 0.375));
    let step_uv_offset = step_texel_offset / vec2f(textureDimensions(color));

    var sum = vec3f(0.0);
    var weight = 0.0;

    for (var step = -half_samples; step < half_samples; step += 1) {
        let distr = math::normal_distribution(f32(step), 0.0, sigma);
        sum += textureSample(color, color_sampler, uv + step_uv_offset * f32(step)).rgb * distr;
        weight += distr;
    }

    return vec4f(sum / weight, 1.0);
}

@fragment
fn gaussian_horizontal(in: FullscreenVertexOutput) -> @location(0) vec4f {
    let coc = calculate_coc_radius(in.uv);
    return gaussian(in.uv, coc, vec2f(1.0, 0.0));
}

@fragment
fn gaussian_vertical(in: FullscreenVertexOutput) -> @location(0) vec4f {
    let coc = calculate_coc_radius(in.uv);
    return gaussian(in.uv, coc, vec2f(0.0, 1.0));
    // return vec4f(coc / 10.0);
}
