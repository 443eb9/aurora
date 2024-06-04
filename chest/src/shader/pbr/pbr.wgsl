struct Camera {
    view: mat4x4f,
    proj: mat4x4f,
}

struct DirectionalLight {
    pos: vec3f,
    dir: vec3f,
    col: vec3f,
}

struct PbrMaterial {
    base_color: vec3f,
    roughness: f32,
    metallic: f32,
}

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec3f,
}

struct VertexOutput {
    @builtin(position) position_cs: vec4f,
    @location(0) position_ws: vec3f,
    @location(1) normal_ws: vec3f,
    @location(2) uv: vec2f,
}

@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var<storage, read> dir_lights: array<DirectionalLight>;

@group(2) @binding(0) var<uniform> material: PbrMaterial;
@group(2) @binding(1) var tex_base_color: texture_2d<f32>;
@group(2) @binding(2) var tex_sampler: sampler;

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position_ws = input.position;
    output.position_cs = camera.proj * camera.view * vec4f(input.position, 1.);
    output.normal_ws = input.normal;
    output.uv = input.uv.xy;
    return output;
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4f {
#ifdef TEX_BASE_COLOR
    var tex_col = textureSample(tex_base_color, tex_sampler, input.uv).rgb;
#else
    var tex_col = vec3f(1.);
#endif

    var light_col = vec3f(0.);

    for (var i_light = 0u; i_light < arrayLength(&dir_lights); i_light += 1u) {
        let light = &dir_lights[i_light];
        light_col += (saturate(dot((*light).dir, input.normal_ws)) * 0.8 + 0.2) * (*light).col;
    }

    let color = material.base_color * light_col * tex_col;
    return vec4f(color, 1.);
}
