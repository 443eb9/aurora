#define_import_path aurora::math

const PI = 3.14159265;

fn sin_between(x: vec3f, y: vec3f) -> f32 {
    return length(cross(x, y)) / (length(x) * length(y));
}

fn cos_between(x: vec3f, y: vec3f) -> f32 {
    return dot(x, y) / (length(x) * length(y));
}

fn clip_to_uv(clip: vec4f) -> vec2f {
    var uv = (clip.xy / clip.w + 1.0) * 0.5;
    uv.y = 1.0 - uv.y;
    return uv;
}

fn square_length(x: vec3f) -> f32 {
    return dot(x, x);
}

fn view_to_uv_and_depth(view: vec3f, proj_mat: mat4x4f) -> vec3f {
    let clip = proj_mat * vec4f(view, 1.0);
    let ndc = clip.xyz / clip.w;
    var uv = (ndc.xy + 1.0) * 0.5;
    uv.y = 1.0 - uv.y;
    return vec3f(uv, ndc.z);
}

fn rotation_mat(angle: f32) -> mat2x2f {
    let s = sin(angle);
    let c = cos(angle);
    return mat2x2f(c, s, -s, c);
}

fn rotate_vector(v: vec2f, angle: f32) -> vec2f {
    return rotation_mat(angle) * v;
}

fn rotate01_vector(v: vec2f, angle: f32) -> vec2f {
    return rotation_mat(angle * 2.0 * PI) * v;
}

fn project_vector_to_plane(v: vec3f, plane_normal: vec3f) -> vec3f {
    return v - dot(v, plane_normal) * plane_normal;
}

fn normal_distribution(x: f32, mean: f32, variance: f32) -> f32 {
    let t = x - mean;
    return 1.0 / sqrt(2.0 * PI * variance & variance) * exp(-(t * t) / (2.0 * variance * variance));
}
