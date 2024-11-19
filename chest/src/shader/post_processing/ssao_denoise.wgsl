#import aurora::math

struct SsaoConfig {
    slices: u32,
    samples: u32,
    strength: f32,
    angle_bias: f32,
    max_depth_diff: f32,
}

@group(0) @binding(0) var<uniform> config: SsaoConfig;
@group(0) @binding(1) var noisy_ao: texture_2d<f32>;
@group(0) @binding(2) var tex_sampler: sampler;
@group(0) @binding(3) var filtered_ao: texture_storage_2d<r32float, write>;

@workgroup_size(#SSAO_WORKGROUP_SIZE, #SSAO_WORKGROUP_SIZE, 1)
@compute
fn main(@builtin(global_invocation_id) id: vec3u) {
    let texel = id.xy;
    if any(texel >= textureDimensions(noisy_ao)) {
        return;
    }
    let uv = vec2f(texel) / vec2f(textureDimensions(noisy_ao));

    let center = textureLoad(noisy_ao, vec2i(texel), 0).r;
    var sum = 0.0;
    var weight = 0.0;

    for (var dx = -5; dx <= 5; dx += 1) {
        for (var dy = -5; dy <= 5; dy += 1) {
            let v = vec2i(dx, dy);
            let x = textureLoad(noisy_ao, vec2i(texel) + v, 0).r;
            let dist = length(vec2f(v));
            let w = math::normal_distribution(dist, 0.0, 0.5) * math::normal_distribution(x - center, 0.0, 0.3);
            weight += w;
            sum += x * w;
        }
    }

    textureStore(filtered_ao, texel, vec4f(pow(sum / weight, f32(config.strength))));
}
