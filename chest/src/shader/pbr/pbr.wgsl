#define_import_path aurora::pbr::pbr
#import aurora::{
    math::PI,
    pbr::{
        pbr_binding::{camera, dir_lights, material, tex_base_color, tex_sampler},
        pbr_function,
        pbr_function::{construct_surface_lit, construct_surface_unlit, apply_exposure},
        pbr_type::{
            Camera, DirectionalLight, PbrMaterial, PbrVertexInput, PbrVertexOutput
        }
    }
    tonemapping,
}

@vertex
fn vertex(input: PbrVertexInput) -> PbrVertexOutput {
    var output: PbrVertexOutput;
    output.position_ws = input.position;
    output.position_cs = camera.proj * camera.view * vec4f(input.position, 1.);
    output.normal_ws = input.normal;
    output.uv = input.uv.xy;
    return output;
}

@fragment
fn fragment(input: PbrVertexOutput) -> @location(0) vec4f {
    var unlit = construct_surface_unlit(input, material, input.uv);

    var color = vec3f(0.);

    for (var i_light = 0u; i_light < arrayLength(&dir_lights); i_light += 1u) {
        let light = &dir_lights[i_light];
        var lit = construct_surface_lit(vec3f(0.), (*light).direction, unlit);

#ifdef GGX
        let D = pbr_function::D_GGX(&unlit, &lit);
        let G = pbr_function::G2_HeightCorrelated(&unlit, &lit);
#else
        let D = 0.;
        let G = 0.;
#endif

#ifdef LAMBERT
        let FD = pbr_function::FD_Lambert(&unlit, &lit);
#else ifdef BURLEY
        let FD = pbr_function::FD_Burley(&unlit, &lit);
#else
        let FD = vec3f(0.);
#endif

        let F = aurora::pbr::pbr_function::F_Schlick(lit.HdotL, unlit.f_normal);

        let f_spec = D * G * F * PI;
        let f_diff = FD * PI;

        color += lit.NdotL * (*light).intensity * (f_spec + f_diff) * (*light).color;
        // color = vec3f(unlit.NdotV);
        // color = input.position_ws;
    }

    color = apply_exposure(color * unlit.base_color);
    return vec4f(tonemapping::tonemapping_tony_mc_mapface(color), 1.);
}
