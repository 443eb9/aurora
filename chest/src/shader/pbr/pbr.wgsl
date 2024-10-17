#define_import_path aurora::pbr::pbr
#import aurora::{
    common_binding::{camera, scene},
    common_type::VertexInput,
    math::PI,
    pbr::{
        pbr_binding::{dir_lights, material, point_lights, spot_lights, tex_base_color, tex_sampler},
        pbr_function,
        pbr_type::PbrVertexOutput,
    }
    shadow_mapping,
    tonemapping,
}

@vertex
fn vertex(in: VertexInput) -> PbrVertexOutput {
    var output: PbrVertexOutput;
    output.position_ws = in.position;
    output.position_vs = camera.view * vec4f(in.position, 1.);
    output.position_cs = camera.proj * output.position_vs;
    output.normal = in.normal;
    output.uv = in.uv.xy;
    output.tangent = in.tangent;
    return output;
}

@fragment
fn fragment(in: PbrVertexOutput) -> @location(0) vec4f {
#ifdef TEX_NORMAL
    let normal = pbr_function::unpack_normal(in.normal, in.tangent, in.uv);
#else
    let normal = in.normal;
#endif

    var unlit = pbr_function::construct_surface_unlit(in.position_ws, normal, in.uv, material);

    var color = vec3f(0.);

    for (var i_light = 0u; i_light < scene.dir_lights; i_light += 1u) {
        let light = &dir_lights[i_light];
        
        let irradiated = pbr_function::apply_lighting((*light).direction, (*light).intensity, (*light).color, &unlit);
        let shadow = shadow_mapping::sample_cascaded_shadow_map(i_light, in.position_ws, in.position_vs);

        color += irradiated * shadow;
#ifdef SHOW_CASCADES
        color += shadow_mapping::debug_cascade_color(i_light, in.position_vs) * (*light).intensity;
#endif // SHOW_CASCADES
    }

    for (var i_light = 0u; i_light < scene.point_lights; i_light += 1u) {
        let light = &point_lights[i_light];
        let position_rel = (*light).position - in.position_ws;
        let direction = normalize(position_rel);
        let d2 = max(dot(position_rel, position_rel), 0.0001);

        let intensity = (*light).intensity / (4. * PI * d2);

        let irradiated = pbr_function::apply_lighting(direction, intensity, (*light).color, &unlit);
        let shadow = shadow_mapping::sample_point_shadow_map(i_light, in.position_ws - (*light).position);

        color += irradiated * shadow;
    }

    for (var i_light = 0u; i_light < scene.spot_lights; i_light += 1u) {
        let light = &spot_lights[i_light];
        let position_rel = (*light).position - in.position_ws;
        let direction = normalize(position_rel);
        let d2 = max(dot(position_rel, position_rel), 0.0001);

        let cos_outer = cos((*light).outer);
        let cos_inner = cos((*light).inner);
        let lambda = max(0., dot(direction, (*light).direction) - cos_outer) / (cos_inner - cos_outer) / PI;

        let intensity = (*light).intensity / (2. * PI * (1. - cos((*light).outer / 2.)) * d2) * lambda;
        // let intensity = (*light).intensity / (PI * dot(position_rel, position_rel)) * lambda;

        let irradiated = pbr_function::apply_lighting(direction, intensity, (*light).color, &unlit);
        let shadow = shadow_mapping::sample_point_shadow_map(i_light, in.position_ws - (*light).position);

        color += irradiated * shadow;
    }

    color = pbr_function::apply_exposure(color * unlit.base_color);
    return vec4f(tonemapping::tonemapping_tony_mc_mapface(color), 1.);
    // return vec4f(color, 1.);
}
