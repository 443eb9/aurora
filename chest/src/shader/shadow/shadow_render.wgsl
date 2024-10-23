#define_import_path aurora::shadow_render
#import aurora::{
    common_binding::camera,
    common_type::VertexInput,
    math,
}

@vertex
fn vertex(in: VertexInput) -> @builtin(position) vec4f {
    var light_dir = vec3f(0.);
    if (camera.proj[3][3] == 1.) {
        light_dir = camera.position; // Orthographic
    } else {
        light_dir = normalize(camera.position - in.position); // Perspective
    }
    let offset = math::sin_between(light_dir, in.normal);
    return camera.proj * camera.view * vec4f(in.position - in.normal * offset, 1.);
}

@fragment
fn fragment() { }
