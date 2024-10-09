#define_import_path aurora::shadow_mapping

#import aurora::common_type::Camera

@group(3) @binding(0) var<storage> light_views: array<Camera>;
@group(3) @binding(1) var directional_shadow_map: texture_depth_2d_array;
// @group(3) @binding(2) var point_shadow_map: texture_depth_cube_array;
// @group(3) @binding(3) var spot_shadow_map: texture_depth_cube_array;
@group(3) @binding(2) var shadow_map_sampler: sampler_comparison;
// @group(3) @binding(2) var shadow_map_sampler: sampler;

fn sample_directional_shadow_map(light: u32, position_ws: vec3f) -> vec2f {
    // Project the mesh point on to light view.
    let position_cs = light_views[light].proj * light_views[light].view * vec4f(position_ws, 1.);
    // Orthographic projection, no need to divide by w.
    var uv = (position_cs.xy + 1.) / 2.;
    uv.y = 1. - uv.y;
    if (uv.x < 0. || uv.x > 1. || uv.y < 0. || uv.y > 1.) {
        return 1.;
    } else {
        return textureSampleCompare(directional_shadow_map, shadow_map_sampler, uv, light, position_cs.z - 0.05);
    }
}

fn sample_point_shadow_map(light: u32, position_ws: vec3f) -> f32 {
    return 1.;
}

fn sample_spot_shadow_map(light: u32, position_ws: vec3f) -> f32 {
    return 1.;
}
