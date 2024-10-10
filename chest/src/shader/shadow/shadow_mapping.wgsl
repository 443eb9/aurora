#define_import_path aurora::shadow_mapping

#import aurora::common_type::Camera

@group(3) @binding(0) var<storage> light_views: array<Camera>;
// @group(3) @binding(1) var shadow_map_sampler: sampler_comparison;
@group(3) @binding(1) var shadow_map_sampler: sampler;
@group(3) @binding(2) var directional_shadow_map: texture_depth_2d_array;
@group(3) @binding(3) var point_shadow_map: texture_depth_cube_array;
@group(3) @binding(4) var spot_shadow_map: texture_depth_cube_array;

fn project_mesh_vertex(light: u32, position_ws: vec3f) -> vec3f {
    // Project the mesh point on to light view.
    let position_cs = light_views[light].proj * light_views[light].view * vec4f(position_ws, 1.);
    let ndc = position_cs.xy / position_cs.w;
    var uv = (ndc + 1.) / 2.;
    uv.y = 1. - uv.y;
    return vec3f(uv, position_cs.z);
}

fn sample_directional_shadow_map(light: u32, position_ws: vec3f) -> f32 {
    let t = project_mesh_vertex(light, position_ws);
    if (t.x < 0. || t.x > 1. || t.y < 0. || t.y > 1.) {
        return 1.;
    } else {
        // return textureSampleCompare(directional_shadow_map, shadow_map_sampler, t.xy, light, saturate(t.z - 0.05));

        let shadow_depth = textureSample(directional_shadow_map, shadow_map_sampler, t.xy, light);
        let frag_depth = saturate(t.z) - 0.001;
        if (frag_depth < shadow_depth) {
            return 1.;
        } else {
            return 0.;
        }
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

    let shadow_depth = textureSample(point_shadow_map, shadow_map_sampler, -relative_pos, light);
    if (projected_depth < shadow_depth) {
        return 1.;
    } else {
        return 0.;
    }
}

fn sample_spot_shadow_map(light: u32, position_ws: vec3f) -> f32 {
    return 1.;
}
