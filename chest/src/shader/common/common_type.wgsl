#define_import_path aurora::common_type

struct Camera {
    view: mat4x4f,
    inv_view: mat4x4f,
    proj: mat4x4f,
    inv_proj: mat4x4f,
    position: vec3f,
    exposure: f32,
}

struct Scene {
    dir_lights: u32,
    point_lights: u32,
    spot_lights: u32,
}

struct DirectionalLight {
    direction: vec3f,
    color: vec3f,
    intensity: f32,
    radius: f32,
}

struct PointLight {
    position: vec3f,
    color: vec3f,
    intensity: f32,
    radius: f32,
}

struct SpotLight {
    position: vec3f,
    direction: vec3f,
    color: vec3f,
    intensity: f32,
    radius: f32,
    inner: f32,
    outer: f32,
}

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) uv: vec2f,
    @location(3) tangent: vec4f,
}
