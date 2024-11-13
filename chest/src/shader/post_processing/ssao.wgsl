#define_import_path aurora::post_processing::ssao
#import aurora::math

#ifdef SSAO

@group(#SSAO) @binding(0) var ssao_texture: texture_2d<f32>;
@group(#SSAO) @binding(1) var ssao_sampler: sampler;

fn get_ao(position_cs: vec4f) -> f32 {
    return textureSample(ssao_texture, ssao_sampler, math::clip_to_uv(position_cs)).r;
}

#endif // SSAO
