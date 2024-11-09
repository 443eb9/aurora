#define_import_path aurora::math

const PI = 3.14159265;

fn sin_between(x: vec3f, y: vec3f) -> f32 {
    return length(cross(x, y)) / (length(x) * length(y));
}

fn view_to_uv_and_depth(view: vec4f, proj_mat: mat4x4f) -> vec3f {
    let clip = proj_mat * view;
    let ndc = clip.xyz / clip.w;
    var uv = (ndc.xy + 1.) * 0.5;
    uv.y = 1. - uv.y;
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
    return rotation_mat(angle * 2. * PI) * v;
}
