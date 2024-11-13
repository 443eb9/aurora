#define_import_path aurora::post_processing::ssao
#import aurora::math

#ifdef SSAO

@group(#SSAO) @binding(0) var ssao_texture: texture_2d<f32>;
@group(#SSAO) @binding(1) var ssao_sampler: sampler;

fn get_ao(uv: vec2f) -> f32 {
    return textureSample(ssao_texture, ssao_sampler, uv).r;
}

#endif // SSAO
