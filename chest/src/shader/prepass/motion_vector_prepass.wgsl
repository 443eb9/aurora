#import aurora::common_type::{Camera, VertexInput}

struct MotionVectorPrepass {
    previous_view: mat4x4f,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<uniform> prepass_data: MotionVectorPrepass;

struct MotionVectorPrepassVertexOutput {
    @builtin(position) position: vec4f,
    @location(0) delta: vec3f,
}

@vertex
fn vertex(in: VertexInput) -> MotionVectorPrepassVertexOutput {
    let previous = prepass_data.previous_view * vec4f(in.position, 1.0);
    let current = camera.view * vec4f(in.position, 1.0);

    var out: MotionVectorPrepassVertexOutput;
    out.position = camera.proj * current;
    out.delta = (current - previous).xyz;
    return out;
}

@fragment
fn fragment(in: MotionVectorPrepassVertexOutput) -> @location(0) vec4f {
    return vec4f(in.delta, 0.0);
}
