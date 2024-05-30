#define_import_path aurora::depth_pass
#import aurora::fullscreen::FullscreenVertexOutput;

@group(0) @binding(0) var depth_texture: texture_depth_2d;
@group(0) @binding(1) var depth_sampler: sampler;

@fragment
fn fragment(input: FullscreenVertexOutput) -> @location(0) vec4f {
    return vec4f(textureSample(depth_texture, depth_sampler, input.uv));
}
