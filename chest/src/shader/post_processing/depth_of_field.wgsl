#import aurora::fullscreen::FullscreenVertexOutput

@fragment
fn gaussian_horizontal(in: FullscreenVertexOutput) -> @location(0) vec4f {
    return vec4f(0.5);
}

@fragment
fn gaussian_vertical(in: FullscreenVertexOutput) -> @location(0) vec4f {
    return vec4f(1.0);
}
