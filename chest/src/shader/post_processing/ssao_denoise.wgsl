struct SsaoConfig {
    texture_dim: vec2u,
    slices: u32,
    samples: u32,
    strength: f32,
    angle_bias: f32,
    max_depth_diff: f32,
}

@group(0) @binding(0) var<uniform> config: SsaoConfig;
@group(0) @binding(1) var noisy_ao: texture_2d<f32>;
@group(0) @binding(2) var ao_sampler: sampler;
@group(0) @binding(3) var filtered_ao: texture_storage_2d<r32float, write>;

@workgroup_size(#SSAO_WORKGROUP_SIZE, #SSAO_WORKGROUP_SIZE, 1)
@compute
fn main(@builtin(global_invocation_id) id: vec3u) {
    let texel = id.xy;
    if any(texel >= config.texture_dim) {
        return;
    }
    let uv = vec2f(texel) / vec2f(config.texture_dim);

    let ao0 = textureGather(0, noisy_ao, ao_sampler, uv);
    let ao1 = textureGather(0, noisy_ao, ao_sampler, uv, vec2i(1, 0));
    let ao2 = textureGather(0, noisy_ao, ao_sampler, uv, vec2i(0, 1));
    let ao3 = textureGather(0, noisy_ao, ao_sampler, uv, vec2i(1, 1));

    let ctr_ao = ao0.y;
    let lft_ao = ao0.x;
    let rht_ao = ao1.x;
    let top_ao = ao0.z;
    let btm_ao = ao2.z;
    let lft_top_ao = ao0.w;
    let rht_top_ao = ao1.w;
    let lft_btm_ao = ao2.w;
    let rht_btm_ao = ao3.w;

    // var sum = ctr_ao;
    // sum += abs(lft_ao - ctr_ao) * lft_ao;
    // sum += abs(rht_ao - ctr_ao) * rht_ao;
    // sum += abs(top_ao - ctr_ao) * top_ao;
    // sum += abs(btm_ao - ctr_ao) * btm_ao;
    // sum += abs(lft_top_ao - ctr_ao) * lft_top_ao / 0.7;
    // sum += abs(rht_top_ao - ctr_ao) * rht_top_ao / 0.7;
    // sum += abs(lft_btm_ao - ctr_ao) * lft_btm_ao / 0.7;
    // sum += abs(rht_btm_ao - ctr_ao) * rht_btm_ao / 0.7;

    // let weight = 4.0 + 0.7 * 4.0;

    // textureStore(filtered_ao, texel, vec4f(sum / weight));

    let sum = ctr_ao + lft_ao + rht_ao + top_ao + btm_ao + lft_top_ao + rht_top_ao + lft_btm_ao + rht_btm_ao;
    let weight = 9.0;
    textureStore(filtered_ao, texel, vec4f(sum / weight));
}
