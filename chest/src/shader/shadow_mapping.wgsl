#define_import_path aurora::shadow_mapping

@group(2) @binding(0) depth_texture: texture_depth_2d;
@group(2) @binding(0) depth_sampler: sampler;

struct ShadowVertexOutput {
    @location(0) position: vec3f,
}

fn vertex(in: ShadowVertexInput) -> ShadowVertexOutput {
    
}

fn fragment() -> @location(0) vec4f {
    return vec4f(0.);
}

fn apply_shadow(in: vec4f) -> vec4f {

}
