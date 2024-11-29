#import aurora::{
    common_type::Camera,
    fullscreen::FullscreenVertexOutput,
}

@group(0) @binding(0) var skybox: texture_cube<f32>;
@group(0) @binding(1) var skybox_sampler: sampler;
@group(0) @binding(2) var<uniform> camera: Camera;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4f {
    let position_ndc = (in.uv * 2.0 - 1.0) * vec2f(1.0, -1.0);
    let position_view = camera.inv_proj * vec4f(position_ndc, 1.0, 1.0);
    let position_world_no_translation = camera.inv_view * vec4f(position_view.xyz / position_view.w, 0.0);

    return textureSample(skybox, skybox_sampler, position_world_no_translation.xyz);
}
