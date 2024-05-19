struct Camera {
    view: mat4x4f,
    proj: mat4x4f,
}

struct DirectionalLight {
    pos: vec3f,
    dir: vec3f,
    col: vec3f,
}

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
}

struct VertexOutput {
    @builtin(position) position_cs: vec4f,
    @location(0) position_ws: vec3f,
    @location(1) normal_ws: vec3f,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<storage, read> dir_lights: array<DirectionalLight>;

@vertex
fn vertex(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position_ws = input.position;
    output.position_cs = camera.proj * camera.view * vec4f(input.position, 1.);
    output.normal_ws = input.normal;
    return output;
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4f {
    var color = vec3f(0.);

    for (var i_light = 0u; i_light < arrayLength(&dir_lights); i_light += 1u) {
        let light = &dir_lights[i_light];
        color += (saturate(dot(normalize((*light).pos - input.position_ws), input.normal_ws)) * 0.5 + 0.5) * (*light).col;
    }

    return vec4f(color, 1.);
}
