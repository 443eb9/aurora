#define_import_path aurora::depth_pass
#import aurora::fullscreen::FullscreenVertexOutput;

@group(0) @binding(0) depth_texture: texture_depth_2d;
@group(0) @binding(1) depth_sampler: sampler;

@fragment
fn fragment(input: FullscreenVertexOutput) -> @location(0) vec4f {
    return vec4f(textureLoad(depth_texture, sampler, input.uv));
}
