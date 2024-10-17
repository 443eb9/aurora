#define_import_path aurora::shadow_mapping

#import aurora::common_type::Camera

@group(3) @binding(0) var<storage> cascade_views: array<Camera>;
@group(3) @binding(1) var<storage> point_light_views: array<Camera>;
@group(3) @binding(2) var shadow_map_sampler: sampler_comparison;
@group(3) @binding(3) var directional_shadow_map: texture_depth_2d_array;
@group(3) @binding(4) var point_shadow_map: texture_depth_cube_array;
@group(3) @binding(5) var<storage> poisson_disk: array<vec2f>;

fn pcf_filtering(position_vs: vec4f, cascade: u32) -> f32 {
    var shadow = 0.;
    for (var iteration = 0u; iteration < #SHADOW_SAMPLE_COUNT; iteration += 1u) {
        let offseted = position_vs + vec4f(poisson_disk[iteration], 0., 0.);
        let position_cs = cascade_views[cascade].proj * offseted;
        let ndc = position_cs.xy / position_cs.w;
        var uv = (ndc + 1.) / 2.;
        uv.y = 1. - uv.y;

        if (uv.x > 0. && uv.x < 1. && uv.y > 0. && uv.y < 1.) {
            let frag_depth = saturate(position_cs.z) - 0.005;
            shadow += textureSampleCompare(directional_shadow_map, shadow_map_sampler, uv, cascade, frag_depth);
        } else {
            shadow += 1.;
        }
    }
    return shadow / f32(#SHADOW_SAMPLE_COUNT);
}

fn no_filtering(position_cs: vec4f, uv: vec2f, cascade: u32) -> f32 {
    let frag_depth = saturate(position_cs.z) - 0.002;
    return textureSampleCompare(directional_shadow_map, shadow_map_sampler, uv, cascade, frag_depth);
}

fn sample_directional_shadow_map(position_vs: vec4f, position_cs: vec4f, uv: vec2f, cascade: u32) -> f32 {
#ifdef PCF
    return pcf_filtering(position_vs, cascade);
#else
    return no_filtering(position_cs, uv, cascade);
#endif
}

fn sample_cascaded_shadow_map(light: u32, position_ws: vec3f, position_vs: vec4f) -> f32 {
    for (var cascade = #SHADOW_CASCADES - 1u; cascade >= 0u; cascade -= 1u) {
        let index = light * #SHADOW_CASCADES + cascade;
        // SPECIAL USE CASE FOR exposure FIELD!!
        // exposure = near plane of this camera.
        // If this point is inside this frustum slice.
        if abs(position_vs.z) > abs(cascade_views[index].exposure) {
            let position_vs = cascade_views[index].view * vec4f(position_ws, 1.);
            let position_cs = cascade_views[cascade].proj * position_vs;
            let ndc = position_cs.xy / position_cs.w;
            var uv = (ndc + 1.) / 2.;
            uv.y = 1. - uv.y;

            if (uv.x > 0. && uv.x < 1. && uv.y > 0. && uv.y < 1.) {
                return sample_directional_shadow_map(position_vs, position_cs, uv, cascade);
            } else {
                return 1.;
            }
        }
    }

    return 1.;
}

fn debug_cascade_color(light: u32, position_vs: vec4f) -> vec3f {
    var CASCADE_COLORS = array<vec3f, 6>(
        vec3f(1., 0., 0.),
        vec3f(0., 1., 0.),
        vec3f(0., 0., 1.),
        vec3f(1., 1., 0.),
        vec3f(1., 0., 1.),
        vec3f(0., 1., 1.),
    );

    for (var cascade = #SHADOW_CASCADES - 1u; cascade >= 0u; cascade -= 1u) {
        let index = light * #SHADOW_CASCADES + cascade;
        // SPECIAL USE CASE FOR exposure FIELD!!
        // exposure = near plane of this camera.
        // If this point is inside this frustum slice.
        if abs(position_vs.z) > abs(cascade_views[index].exposure) {
            return CASCADE_COLORS[cascade % 6];
        }
    }

    return vec3f(1.);
    // return vec3f(abs(cascade_views[1].exposure));
    // return vec3f(f32(arrayLength(&cascade_views)) / 3.);
}

fn sample_point_shadow_map(light: u32, relative_pos: vec3f) -> f32 {
    // Find the axis with largest absolute value.
    let abs_pos = abs(relative_pos);
    let frag_depth = -max(abs_pos.x, max(abs_pos.y, abs_pos.z));

    // Do a simple projection.
    let proj = point_light_views[light].proj;
    let v = vec2f(frag_depth * proj[2][2] + proj[3][2], -frag_depth);
    let projected_depth = v.x / v.y - 0.001;

    return textureSampleCompare(point_shadow_map, shadow_map_sampler, -relative_pos, light, projected_depth);
}
