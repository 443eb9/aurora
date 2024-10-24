#define_import_path aurora::shadow_mapping

#import aurora::{common_type::Camera, math}

struct ShadowMappingConfig {
    dir_map_resolution: u32,
    point_map_resolution: u32,
    samples: u32,
    pcf_radius: f32,
    pcss_radius: f32,
}

@group(3) @binding(0) var<storage> cascade_views: array<Camera>;
@group(3) @binding(1) var<storage> point_light_views: array<Camera>;
@group(3) @binding(2) var shadow_map_sampler: sampler_comparison;
@group(3) @binding(3) var shadow_texture_sampler: sampler;
@group(3) @binding(4) var directional_shadow_map: texture_depth_2d_array;
@group(3) @binding(5) var point_shadow_map: texture_depth_cube_array;
@group(3) @binding(6) var<storage> poisson_disk: array<vec2f>;
@group(3) @binding(7) var<uniform> config: ShadowMappingConfig;

fn pcf_filtering(position_vs: vec4f, cascade: u32, radius: f32) -> f32 {
    var shadow = 0.;
    for (var iteration = 0u; iteration < config.samples; iteration += 1u) {
        let view = position_vs + vec4f(poisson_disk[iteration] * radius, 0., 0.);
        var offseted = math::view_to_uv_and_depth(view, cascade_views[cascade].proj);

        if (offseted.x > 0. && offseted.x < 1. && offseted.y > 0. && offseted.y < 1.) {
            let frag_depth = saturate(offseted.z) - 0.005;
            shadow += textureSampleCompare(directional_shadow_map, shadow_map_sampler, offseted.xy, cascade, frag_depth);
        } else {
            shadow += 1.;
        }
    }
    return shadow / f32(config.samples);
}

// fn pcss_filtering(position_cs: vec4f, uv: vec2f, cascade: u32, light_width: f32, radius: f32) -> f32 {
//     var avg_blocker_depth = 0.;
//     let texel = 1. / f32(config.dir_map_resolution);
//     let frag_depth = saturate(position_cs.z / position_cs.w) - 0.002;
//     var cnt = 0;
//     for (var iteration = 0u; iteration < config.samples; iteration += 1u) {
//         let offseted = uv + poisson_disk[iteration] * radius * texel;

//         if (uv.x > 0. && uv.x < 1. && uv.y > 0. && uv.y < 1.) {
//             let shadow_depth = textureSample(directional_shadow_map, shadow_texture_sampler, offseted, cascade);
//             if (frag_depth > shadow_depth) {
//                 avg_blocker_depth += shadow_depth;
//                 cnt += 1;
//             }
//         }
//     }
//     avg_blocker_depth /= f32(max(cnt, 1));

//     let receiver_depth = position_cs.z / position_cs.w;
//     let penumbra = max(receiver_depth - avg_blocker_depth, 0.) / avg_blocker_depth * light_width;

//     return pcf_filtering(position_cs, uv, cascade, penumbra);
// }

fn no_filtering(uv: vec2f, depth: f32, cascade: u32) -> f32 {
    let frag_depth = saturate(depth) - 0.001;
    return textureSampleCompare(directional_shadow_map, shadow_map_sampler, uv, cascade, frag_depth);
}

fn sample_cascaded_shadow_map(light: u32, position_ws: vec3f, position_vs: vec4f, light_width: f32) -> f32 {
    for (var cascade = #SHADOW_CASCADES - 1u; cascade >= 0u; cascade -= 1u) {
        let index = light * #SHADOW_CASCADES + cascade;
        // SPECIAL USE CASE FOR exposure FIELD!!
        // exposure = near plane of this camera.
        // If this point is inside this frustum slice.
        if abs(position_vs.z) > abs(cascade_views[index].exposure) {
            let position_vs = cascade_views[index].view * vec4f(position_ws, 1.);
            let uv_and_depth = math::view_to_uv_and_depth(position_vs, cascade_views[index].proj);

            if (uv_and_depth.x > 0. && uv_and_depth.x < 1. && uv_and_depth.y > 0. && uv_and_depth.y < 1.) {
                #ifdef PCF
                    return pcf_filtering(position_vs, cascade, config.pcf_radius);
                // #else ifdef PCSS
                //     return pcss_filtering(position_cs, uv, cascade, light_width, config.pcss_radius);
                #else
                    return no_filtering(uv_and_depth.xy, uv_and_depth.z, cascade);
                #endif
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
