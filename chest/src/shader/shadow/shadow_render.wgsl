#define_import_path aurora::shadow_render
#import aurora::{
    common_binding::camera,
    common_type::VertexInput,
}

@vertex
fn vertex(in: VertexInput) -> @builtin(position) vec4f {
    return camera.proj * camera.view * vec4f(in.position, 1.);
}

@fragment
fn fragment() { }
