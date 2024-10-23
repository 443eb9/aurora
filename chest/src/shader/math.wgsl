#define_import_path aurora::math

const PI = 3.14159265;

fn sin_between(x: vec3f, y: vec3f) -> f32 {
    return length(cross(x, y)) / (length(x) * length(y));
}
