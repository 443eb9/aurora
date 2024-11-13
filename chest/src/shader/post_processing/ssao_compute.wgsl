#import aurora::{common_type::Camera, hash, math, math::PI}

struct SsaoConfig {
    texture_dim: vec2u,
    slices: u32,
    samples: u32,
}

@group(0) @binding(0) var depth: texture_depth_2d;
@group(0) @binding(1) var normal: texture_2d<f32>;
// @group(0) @binding(2) var output: texture_storage_2d<r32float, write>;
@group(0) @binding(2) var output: texture_storage_2d<rgba32float, write>;
@group(0) @binding(3) var<uniform> config: SsaoConfig;
@group(0) @binding(4) var tex_sampler: sampler;
@group(0) @binding(5) var<uniform> camera: Camera;

const STEP_LENGTH: f32 = 0.05;

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

@workgroup_size(#WORKGROUP_SIZE, #WORKGROUP_SIZE, 1)
@compute
fn main(@builtin(global_invocation_id) id: vec3u) {
    let texel = id.xy;
    if id.x >= config.texture_dim.x || id.y > config.texture_dim.y {
        return;
    }
    let tex_sizef = vec2f(config.texture_dim);
    let uv = vec2f(texel) / tex_sizef;

    let texel_depth = view_space_depth(uv);
    // Convert all points into view space.
    let position_vs = view_space_position(uv);
    let normal_vs = view_space_normal(uv);
    // Direction from point to camera.
    let view_dir = normalize(-position_vs);
    // Random rotation to avoid artifact.
    // let randomness = hash::hash12u(texel) * 2.0 * PI;
    let randomness = 0.0;

    var ao = 0.0;
    var another = 0.0;

    for (var slice_index = 0u; slice_index < config.slices; slice_index += 1u) {
        // Get the direction of current slice, in view space.
        let angle = (f32(slice_index) / f32(config.slices) + randomness) * 2.0 * PI;
        let dir = vec2f(cos(angle), sin(angle));
        let dir3 = vec3f(dir, 0.0);

        // Horizon angle, the angle between the sample direction and the vector from point to
        // the highest point along this sample direction.
        var horizon = 0.0;
        var sin_horizon_angle = 0.0;
        // Tangent angle, the angle between the sample direction and the tangent at this point
        // in view space.
        // let bitangent_vs = cross(normal_vs, dir3);
        // let tangent_vs = cross(bitangent_vs, normal_vs);
        // let sin_tangent_angle = math::sin_between(dir3, tangent_vs);
        let sin_tangent_angle = math::sin_between(dir3, math::project_vector_to_plane(dir3, normal_vs));

        for (var sample_index = 1u; sample_index <= config.samples; sample_index += 1u) {
            // March in the sample direction, in view space.
            let sample_vs = position_vs + dir3 * f32(sample_index) * STEP_LENGTH;
            // Get the depth at this sample position.
            let sample_depth = view_space_depth(math::view_to_uv_and_depth(sample_vs, camera.proj).xy);

            // Height difference. We only cares about those points that are higher than the original
            // point, and they are closer to the camera, having lower value of depth.
            let diff = texel_depth - sample_depth;
            
            // Find the highest point.
            if diff > horizon {
                horizon = diff;
                let dist = f32(sample_index) * STEP_LENGTH;
                sin_horizon_angle = horizon / sqrt(horizon * horizon + dist * dist);
                // sin_horizon_angle = sin(atan(horizon / dist));
                // sin_horizon_angle = horizon;
            }
        }

        ao += sin_horizon_angle - sin_tangent_angle;
        // ao += sin_tangent_angle - sin_horizon_angle;
        // ao += sin_tangent_angle;
        // ao += sin_horizon_angle;
        // another += sin_tangent_angle;
        // ao += sample_depth;
        // ao += math::sin_between(dir3, math::project_vector_to_plane(dir3, normal_vs));

        // textureStore(output, id.xy, vec4f(math::sin_between(dir3, math::project_vector_to_plane(dir3, normal_vs))));
        // textureStore(output, id.xy, vec4f(abs(normal_vs.x)));
        // textureStore(output, id.xy, vec4f(dir, 0.0, 1.0));
        // textureStore(output, id.xy, vec4f(textureSampleLevel(normal, tex_sampler, uv, 0.0) * 2.0 - 1.0));
        // textureStore(output, id.xy, vec4f(normal_vs, 0.));
        // textureStore(output, id.xy, vec4f(position_vs, 0.0));
        // textureStore(output, id.xy, vec4f(sin_tangent_angle));

        // if (slice_index == 0u) {
        //     break;
        // }
    }

    ao /= f32(config.slices * config.samples);
    // another /= f32(config.slices * config.samples);
    // ao = clamp(ao, 0.03, 1.0);
    ao = saturate(ao);
    // another = saturate(another);

    textureStore(output, id.xy, vec4f(ao));
    // textureStore(output, id.xy, vec4f(ao, another, 0.0, 1.0));
    // textureStore(output, id.xy, vec4f(texel_depth));
    // textureStore(output, id.xy, vec4f(position_vs, 1.));
}
