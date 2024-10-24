#define_import_path aurora::shadow_render
#import aurora::{
    common_binding::camera,
    common_type::VertexInput,
    math,
    shadow_type::ShadowMappingConfig,
}

@group(0) @binding(1) var<uniform> config: ShadowMappingConfig;

@vertex
fn vertex(in: VertexInput) -> @builtin(position) vec4f {
    var offset = 0.;
    if (camera.proj[3][3] == 1.) {
        offset = math::sin_between(camera.position, in.normal) * (204.8 / f32(config.dir_map_resolution));
    } else {
        offset = math::sin_between(camera.position - in.position, in.normal) * (12.8 / f32(config.point_map_resolution));
    }
    return camera.proj * camera.view * vec4f(in.position - offset * in.normal, 1.);
}

@fragment
fn fragment() { }
