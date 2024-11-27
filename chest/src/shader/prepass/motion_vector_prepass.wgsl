#import aurora::{
    common_type::{Camera, VertexInput},
    math,
}

struct MotionVectorPrepassConfig {
    previous_view: mat4x4f,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<uniform> config: MotionVectorPrepassConfig;

struct MotionVectorPrepassVertexOutput {
    @builtin(position) position: vec4f,
    @location(0) current_position: vec4f,
    @location(1) previous_position: vec4f,
}

@vertex
fn vertex(in: VertexInput) -> MotionVectorPrepassVertexOutput {
    let current = camera.view * vec4f(in.position, 1.0);
    let previous = config.previous_view * vec4f(in.position, 1.0);

    var out: MotionVectorPrepassVertexOutput;
    out.position = camera.proj * current;
    out.current_position = out.position;
    out.previous_position = camera.proj * previous;
    return out;
}

@fragment
fn fragment(in: MotionVectorPrepassVertexOutput) -> @location(0) vec4f {
    let uv = math::clip_to_uv(in.current_position);
    let previous_uv = math::clip_to_uv(in.previous_position);
    return vec4f((uv - previous_uv) * 2.0, 0.0, 0.0);
}
