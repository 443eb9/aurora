#define_import_path aurora::fullscreen

struct FullscreenVertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

@group(0) @binding(0) var color: texture_2d<f32>;
@group(0) @binding(1) var color_sampler: sampler;

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> FullscreenVertexOutput {
    var output: FullscreenVertexOutput;
    let t = vec2f(f32(vertex_index / 2u), f32(vertex_index % 2u));
    output.position = vec4f(vec2f(t * 4. - 1.), 0., 1.);
    output.uv = t * 2.0;
    output.uv.y = 1.0 - output.uv.y;
    return output;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4f {
    return textureSample(color, color_sampler, in.uv);
}
