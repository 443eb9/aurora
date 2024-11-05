#import aurora::{
    env_mapping::env_mapping_type::EnvironmentMapConvolution,
    // TODO: directly import once supported
    // fullscreen::{FullscreenVertexOutput, vertex},
    fullscreen::FullscreenVertexOutput,
    math::PI,
}

struct CubeMapFace {
    view: mat4x4f,
    up: vec3f,
}

@group(0) @binding(0) var env_map: texture_cube<f32>;
@group(0) @binding(1) var env_sampler: sampler;
@group(0) @binding(2) var<uniform> config: EnvironmentMapConvolution;
@group(0) @binding(3) var<uniform> sample_face: CubeMapFace;

@vertex
fn vertex(@builtin(vertex_index) vertex_index: u32) -> FullscreenVertexOutput {
    var output: FullscreenVertexOutput;
    let t = vec2f(f32(vertex_index / 2u), f32(vertex_index % 2u));
    output.position = vec4f(vec2f(t * 4. - 1.), 0., 1.);
    output.uv = t * 2.;
    return output;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4f {
    let texel = (in.uv * 2. - 1.) * config.sample_distance;
    let normal = normalize((sample_face.view * vec4f(-texel, config.sample_distance, 0.)).xyz);

    let up = vec3f(0., 1., 0.);
    // let up = sample_face.up;
    let tangent = normalize(cross(up, normal));
    let bitangent = normalize(cross(normal, tangent));
    let ttw = mat3x3f(tangent, bitangent, normal);

    var irradiance = vec3f(0.);
    for (var azimuth = 1u; azimuth <= config.azimuth_samples; azimuth += 1u) {
        for (var elevation = 0u; elevation < config.elevation_samples; elevation += 1u) {
            let azim = f32(azimuth) / f32(config.azimuth_samples) * 2. * PI;
            let elev = f32(elevation) / f32(config.elevation_samples) * 0.5 * PI;
            let sample_ts = vec3f(sin(elev) * cos(azim), sin(elev) * sin(azim), cos(elev));
            let sample_ws = ttw * sample_ts;

            // irradiance += textureSample(env_map, env_sampler, sample_ws).rgb * sin(elev) * cos(elev);
            irradiance += textureSample(env_map, env_sampler, sample_ws).rgb;
            // irradiance += sample_ws;
        }
    }

    return vec4f(PI * irradiance / f32(config.elevation_samples * config.azimuth_samples), 1.);
    // return vec4f(irradiance, 1.);
    // return vec4f(bitangent, 1.);
    // return normalize(vec4f(-texel, config.sample_distance, 1.));
}
