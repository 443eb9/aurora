#define_import_path aurora::shadow_mapping

#import aurora::common_type::Camera

@group(3) @binding(0) var<storage> light_views: array<Camera>;
@group(3) @binding(1) var shadow_map_sampler: sampler_comparison;
// @group(3) @binding(1) var shadow_map_sampler: sampler;
@group(3) @binding(2) var directional_shadow_map: texture_depth_2d_array;
@group(3) @binding(3) var point_shadow_map: texture_depth_cube_array;

fn sample_directional_shadow_map(light: u32, position_ws: vec3f) -> f32 {
    // Project the mesh point on to light view.
    let position_cs = light_views[light].proj * light_views[light].view * vec4f(position_ws, 1.);
    let ndc = position_cs.xy / position_cs.w;
    var uv = (ndc + 1.) / 2.;
    uv.y = 1. - uv.y;
    if (uv.x < 0. || uv.x > 1. || uv.y < 0. || uv.y > 1.) {
        return 1.;
    } else {
        let frag_depth = saturate(position_cs.z) - 0.05;
        return textureSampleCompare(directional_shadow_map, shadow_map_sampler, uv, light, frag_depth);
    }
}

fn sample_point_shadow_map(light: u32, relative_pos: vec3f) -> f32 {
    // Find the axis with largest absolute value.
    let abs_pos = abs(relative_pos);
    let frag_depth = -max(abs_pos.x, max(abs_pos.y, abs_pos.z));

    // Do a simple projection.
    let proj = light_views[light].proj;
    let v = vec2f(frag_depth * proj[2][2] + proj[3][2], -frag_depth);
    let projected_depth = v.x / v.y - 0.001;

    return textureSampleCompare(point_shadow_map, shadow_map_sampler, -relative_pos, light, projected_depth);
}
