#define_import_path aurora::pbr::pbr
#import aurora::{
    math::PI,
    pbr::{
        pbr_binding::{camera, dir_lights, material, point_lights, spot_lights, tex_base_color, tex_sampler},
        pbr_function::{apply_exposure, apply_lighting, construct_surface_unlit},
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

    // Subtract array length by one as there're dummy data.

    for (var i_light = 0u; i_light < arrayLength(&dir_lights) - 1u; i_light += 1u) {
        let light = &dir_lights[i_light];
        color += apply_lighting((*light).direction, (*light).intensity, (*light).color, &unlit);
    }

    for (var i_light = 0u; i_light < arrayLength(&point_lights) - 1u; i_light += 1u) {
        let light = &point_lights[i_light];
        let position_rel = (*light).position - input.position_ws;
        let direction = normalize(position_rel);
        let d2 = max(dot(position_rel, position_rel), 0.0001);

        let intensity = (*light).intensity / (4. * PI * d2);
        color += apply_lighting(direction, intensity, (*light).color, &unlit);
    }

    for (var i_light = 0u; i_light < arrayLength(&spot_lights) - 1u; i_light += 1u) {
        let light = &spot_lights[i_light];
        let position_rel = (*light).position - input.position_ws;
        let direction = normalize(position_rel);
        let d2 = max(dot(position_rel, position_rel), 0.0001);

        let cos_outer = cos((*light).outer);
        let cos_inner = cos((*light).inner);
        let lambda = max(0., dot(direction, (*light).direction) - cos_outer) / (cos_inner - cos_outer) / PI;

        let intensity = (*light).intensity / (2. * PI * (1. - cos((*light).outer / 2.)) * d2) * lambda;
        // let intensity = (*light).intensity / (PI * dot(position_rel, position_rel)) * lambda;
        color += apply_lighting(direction, intensity, (*light).color, &unlit);
    }

    color = apply_exposure(color * unlit.base_color);
    return vec4f(tonemapping::tonemapping_tony_mc_mapface(color), 1.);
}
