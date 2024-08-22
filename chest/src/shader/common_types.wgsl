#define_import_path aurora::common_types

struct Camera {
    view: mat4x4f,
    proj: mat4x4f,
    position: vec3f,
    exposure: f32,
}

struct DirectionalLight {
    direction: vec3f,
    color: vec3f,
    intensity: f32,
}

struct PointLight {
    position: vec3f,
    color: vec3f,
    intensity: f32,
}

struct SpotLight {
    position: vec3f,
    direction: vec3f,
    color: vec3f,
    intensity: f32,
    inner: f32,
    outer: f32,
}
