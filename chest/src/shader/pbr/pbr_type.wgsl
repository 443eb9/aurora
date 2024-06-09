#define_import_path aurora::pbr::pbr_type

struct Camera {
    view: mat4x4f,
    proj: mat4x4f,
    position_ws: vec3f,
    exposure: f32,
}

struct DirectionalLight {
    positioin_ws: vec3f,
    direction: vec3f,
    color: vec3f,
    intensity: f32,
}

struct PbrMaterial {
    base_color: vec3f,
    roughness: f32,
    metallic: f32,
    ior: f32,
}

struct PbrVertexInput {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec3f,
}

struct PbrVertexOutput {
    @builtin(position) position_cs: vec4f,
    @location(0) position_ws: vec3f,
    @location(1) normal_ws: vec3f,
    @location(2) uv: vec2f,
}

struct BrdfSurfaceUnlit {
    base_color: vec3f,
    roughness: f32,
    metallic: f32,

    normal: vec3f,
    view: vec3f,

    f_normal: f32,

    NdotV: f32,
}

struct BrdfSurfaceLit {
    light: vec3f,
    half: vec3f,

    NdotL: f32,
    NdotH: f32,
    HdotL: f32,
}
