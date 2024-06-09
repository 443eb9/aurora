#define_import_path aurora::pbr::pbr
#import aurora::{
    math::PI,
    pbr::{
        pbr_binding::{camera, dir_lights, material, point_lights, tex_base_color, tex_sampler},
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

    for (var i_light = 0u; i_light < arrayLength(&dir_lights); i_light += 1u) {
        let light = &dir_lights[i_light];
        color += apply_lighting((*light).direction, (*light).intensity, (*light).color, &unlit);
    }

    for (var i_light = 0u; i_light < arrayLength(&point_lights); i_light += 1u) {
        let light = &point_lights[i_light];
        let position_rel = (*light).position - input.position_ws;
        let direction = normalize(position_rel);
        let intensity = (*light).intensity / dot(position_rel, position_rel);
        color += apply_lighting(direction, intensity, (*light).color, &unlit);
    }

    color = apply_exposure(color * unlit.base_color);
    return vec4f(tonemapping::tonemapping_tony_mc_mapface(color), 1.);
}
