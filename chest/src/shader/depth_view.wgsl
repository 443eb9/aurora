#define_import_path aurora::depth_pass
#import aurora::fullscreen::FullscreenVertexOutput;

@group(0) @binding(0) var depth_texture: texture_depth_2d;
@group(0) @binding(1) var depth_sampler: sampler;

@fragment
fn fragment(input: FullscreenVertexOutput) -> @location(0) vec4f {
    let uv = vec2f(input.uv.x, 1. - input.uv.y);
    return vec4f(pow(textureSample(depth_texture, depth_sampler, uv), 4.));
}
