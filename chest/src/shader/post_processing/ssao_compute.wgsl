#import aurora::{common_type::Camera, hash, math, math::PI}

struct SsaoConfig {
    texture_dim: vec2u,
    slices: u32,
    samples: u32,
    strength: f32,
    angle_bias: f32,
    max_depth_diff: f32,
}

@group(0) @binding(0) var depth: texture_depth_2d;
@group(0) @binding(1) var normal: texture_2d<f32>;
@group(0) @binding(2) var output: texture_storage_2d<r32float, write>;
@group(0) @binding(3) var<uniform> config: SsaoConfig;
@group(0) @binding(4) var tex_sampler: sampler;
@group(0) @binding(5) var<uniform> camera: Camera;
@group(0) @binding(6) var hilbert_lut: texture_2d<u32>;

const STEP_LENGTH: f32 = 0.02;

fn view_space_normal(uv: vec2f) -> vec3f {
    let normal_ws = textureSampleLevel(normal, tex_sampler, uv, 0.0).xyz;
    let view_mat = mat3x3f(
        camera.view[0].xyz,
        camera.view[1].xyz,
        camera.view[2].xyz,
    );
    return view_mat * (normal_ws * 2.0 - 1.0);
}

fn view_space_position(uv: vec2f) -> vec3f {
    let clip = vec2f(uv.x * 2.0 - 1.0, 1.0 - 2.0 * uv.y);
    let t = camera.inv_proj * vec4f(clip, frag_depth(uv), 1.0);
    return t.xyz / t.w;
}

fn frag_depth(uv: vec2f) -> f32 {
    return textureSampleLevel(depth, tex_sampler, uv, 0.0);
}

fn view_space_depth(uv: vec2f) -> f32 {
    return -view_space_position(uv).z;
}

@workgroup_size(#SSAO_WORKGROUP_SIZE, #SSAO_WORKGROUP_SIZE, 1)
@compute
fn main(@builtin(global_invocation_id) id: vec3u) {
    let texel = id.xy;
    if any(id.xy >= config.texture_dim) {
        return;
    }
    let tex_sizef = vec2f(config.texture_dim);
    let uv = vec2f(texel) / tex_sizef;

    // Convert all data into view space.
    let texel_depth = view_space_depth(uv);
    let texel_vs = view_space_position(uv);
    let normal_vs = view_space_normal(uv);
    // Direction from point to camera.
    let view_dir = normalize(-texel_vs);
    // Random rotation to avoid artifact.
    // let randomness = hash::hash12u(texel) * 2.0 * PI;
    let randomness = math::hilbert_curve_noise(textureLoad(hilbert_lut, texel % 64, 0).r);

    var ao = 0.0;

    for (var slice_index = 0u; slice_index < config.slices; slice_index += 1u) {
        // Get the direction of current slice, in view space.
        let angle = ((f32(slice_index) + randomness.x) / f32(config.slices)) * 2.0 * PI;
        let dir = vec2f(cos(angle), sin(angle));
        let dir3 = vec3f(dir, 0.0);

        // Horizon angle, the angle between the sample direction and the vector from point to
        // the highest point along this sample direction.
        var sin_horizon_angle = 0.0;
        // Tangent angle, the angle between the sample direction and the tangent at this point
        // in view space.
        let sin_tangent_angle = math::sin_between(dir3, math::project_vector_to_plane(dir3, normal_vs));
        let tangent_angle = asin(sin_tangent_angle);

        var weighted_ao = 0.0;
        var nonweighted_ao = 0.0;

        for (var sample_index = 1u; sample_index <= config.samples; sample_index += 1u) {
            // March in the sample direction, in view space.
            var planar_dist = (f32(sample_index) + randomness.y) * STEP_LENGTH;
            let sample_vs = texel_vs + dir3 * planar_dist;

            // Get the depth at this sample position.
            let sample_depth = view_space_depth(math::view_to_uv_and_depth(sample_vs, camera.proj).xy);

            // Height difference. We only cares about those points that are higher than the original
            // point, and they are closer to the camera, having lower value of depth.
            let diff = texel_depth - sample_depth;
            let sin_angle = diff / sqrt(diff * diff + planar_dist * planar_dist);
            let horizon_angle = asin(sin_angle);
            
            // Find the highest point.
            if sin_angle > sin_horizon_angle && horizon_angle > config.angle_bias && diff < config.max_depth_diff {
                let t = f32(sample_index - 1u) / f32(config.samples);
                let sample_weight = 1.0 - t * t;

                sin_horizon_angle = sin_angle;
                weighted_ao += sample_weight * (sin_horizon_angle - sin_tangent_angle - nonweighted_ao);
                nonweighted_ao = sin_horizon_angle - sin_tangent_angle;
            }
        }

        ao += weighted_ao;
    }

    ao /= f32(config.slices * config.samples);
    // ao = pow(1.0 - saturate(ao * config.strength), config.strength);
    ao = 1.0 - saturate(ao);

    textureStore(output, id.xy, vec4f(ao, 0.0, 0.0, 0.0));
}
