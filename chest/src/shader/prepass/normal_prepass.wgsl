#define_import_path aurora::prepass::normal_prepass
#import aurora::common_type::{Camera, VertexInput}

@group(0) @binding(0) var<uniform> camera: Camera;

struct VertexOutput {
    @builtin(position) position_cs: vec4f,
    @location(0) normal_ws: vec3f,
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position_cs = camera.proj * camera.view * vec4f(in.position, 1.);
    out.normal_ws = normalize(in.normal);
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4f {
    return vec4f(in.normal_ws * 0.5 + 0.5, 1.);
}
