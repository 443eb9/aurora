#define_import_path aurora::pbr::pbr_type

struct PbrMaterial {
    base_color: vec3f,
    roughness: f32,
    metallic: f32,
    reflectance: f32,
}

struct PbrVertexInput {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec2f,
    @location(3) tangent: vec4f,
}

struct PbrVertexOutput {
    @builtin(position) position_cs: vec4f,
    @location(0) position_ws: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec2f,
    @location(3) tangent: vec4f,
}

struct BrdfSurfaceUnlit {
    base_color: vec3f,
    roughness: f32,
    metallic: f32,

    normal: vec3f,
    view: vec3f,

    f_normal: vec3f,

    NdotV: f32,
}

struct BrdfSurfaceLit {
    light: vec3f,
    half: vec3f,

    NdotL: f32,
    NdotH: f32,
    HdotL: f32,
}
