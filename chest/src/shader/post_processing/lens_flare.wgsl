#import aurora::{
    fullscreen::FullscreenVertexOutput,
    hash,
    math,
}

struct LensFlareConfig {
    steps: u32,
    step_factor: f32,
    center_falloff: f32,
    luminance_gain: f32,
    ca_strength: f32,
    halo_radius: f32,
}

@group(0) @binding(0) var color: texture_2d<f32>;
@group(0) @binding(1) var color_sampler: sampler;
@group(0) @binding(2) var<uniform> config: LensFlareConfig;
@group(0) @binding(3) var startburst_texture: texture_1d<f32>;

fn chromatic_aberration(uv: vec2f, dir: vec2f, strength: vec3f) -> vec3f {
    let r = textureSample(color, color_sampler, uv + dir * strength.r).r;
    let g = textureSample(color, color_sampler, uv + dir * strength.g).g;
    let b = textureSample(color, color_sampler, uv + dir * strength.b).b;
    return vec3f(r, g, b);
}

fn startburst_factor(dir: vec2f) -> f32 {
    var angle = acos(math::cos_between_2d(dir, vec2f(1.0, 0.0)));
    if dir.y < 0.0 {
        angle += math::PI;
    }
    return textureSample(startburst_texture, color_sampler, angle / (2.0 * math::PI)).r;
}

fn halo(uv: vec2f) -> vec3f {
    let dir = uv - vec2f(0.5);
    let dist = length(dir);
    let strength = pow(1.0 - abs(dist - config.halo_radius), 50.0);
#ifdef CHROMATIC_ABERRATION
    var col = chromatic_aberration(uv, dir / vec2f(textureDimensions(color)), vec3f(-config.ca_strength, 0.0, config.ca_strength));
#else // CHROMATIC_ABERRATION
    var col = textureSample(color, color_sampler, uv).rgb;
#endif // CHROMATIC_ABERRATION

#ifdef STAR_BURST
    col *= startburst_factor(dir);
#endif // STAR_BURST

    return col * strength;
}

@fragment
fn blit(in: FullscreenVertexOutput) -> @location(0) vec4f {
    return textureSample(color, color_sampler, in.uv);
}

@fragment
fn lens_flare(in: FullscreenVertexOutput) -> @location(0) vec4f {
    let dim = vec2f(textureDimensions(color));
    let texel = 1.0 / dim;
    let flipped_uv = 1.0 - in.uv;

    let dir = (vec2f(0.5) - flipped_uv) * texel;
    let step_length = config.step_factor * dir;

    var col = vec3f(0.0);
    for (var i = 0; i < i32(config.steps); i += 1) {
        let sample_uv = flipped_uv + step_length * f32(i);

#ifdef CHROMATIC_ABERRATION
        let pixel = chromatic_aberration(sample_uv, dir, vec3f(-config.ca_strength, 0.0, config.ca_strength));
#else // CHROMATIC_ABERRATION
        let pixel = textureSample(color, color_sampler, sample_uv).rgb;
#endif // CHROMATIC_ABERRATION

        let luminance = math::luminance(math::linear_to_srgb(pixel));
        let weight = length(vec2f(0.5) - sample_uv) / length(vec2f(0.5));
        col += pixel * pow(1.0 - weight, config.center_falloff) * pow(luminance, config.luminance_gain);
    }

#ifdef HALO
    col += halo(flipped_uv);
#endif // HALO

    let luminance = math::luminance(math::linear_to_srgb(col));
    return vec4f(col, luminance);
}
