#define_import_path aurora::shadow_render
#import aurora::{
    common_binding::camera,
    common_type::VertexInput,
}

struct ShadowVertexOutput {
    @builtin(position) position_cs: vec4f,
    @location(0) position_ws: vec3f,
}

@vertex
fn vertex(in: VertexInput) -> ShadowVertexOutput {
    var out: ShadowVertexOutput;
    out.position_cs = camera.proj * camera.view * vec4f(in.position, 1.);
    out.position_ws = in.position;
    return out;
}

@fragment
fn fragment() { }
