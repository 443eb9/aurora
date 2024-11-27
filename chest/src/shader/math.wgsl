#define_import_path aurora::math

const PI = 3.14159265;

fn sin_between(x: vec3f, y: vec3f) -> f32 {
    return length(cross(x, y)) / (length(x) * length(y));
}

fn sin_between_2d(x: vec2f, y: vec2f) -> f32 {
    return length(cross(x, y)) / (length(x) * length(y));
}

fn cos_between(x: vec3f, y: vec3f) -> f32 {
    return dot(x, y) / (length(x) * length(y));
}

fn cos_between_2d(x: vec2f, y: vec2f) -> f32 {
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
    return 1.0 / sqrt(2.0 * PI * variance * variance) * exp(-(t * t) / (2.0 * variance * variance));
}

// https://extremelearning.com.au/unreasonable-effectiveness-of-quasirandom-sequences/
fn hilbert_curve_noise(index: u32) -> vec2f {
    return fract(0.5 + f32(index) * vec2<f32>(0.75487766624669276005, 0.5698402909980532659114));
}

// Bevy
// https://blog.demofox.org/2022/01/01/interleaved-gradient-noise-a-different-kind-of-low-discrepancy-sequence
fn interleaved_gradient_noise(pixel_coordinates: vec2<f32>, frame: u32) -> f32 {
    let xy = pixel_coordinates + 5.588238 * f32(frame % 64u);
    return fract(52.9829189 * fract(0.06711056 * xy.x + 0.00583715 * xy.y));
}

fn clip_depth_to_view(depth: f32, inv_proj: mat4x4f) -> f32 {
    let t = inv_proj * vec4f(0.0, 0.0, depth, 1.0);
    return -t.z / t.w;
}

fn linear_to_srgb(color: vec3f) -> vec3f {
    return pow(color, vec3f(1.0 / 2.2));
}

fn luminance(c: vec3f) -> f32 {
    return c.r * 0.2126 + c.g * 0.7152 + c.b * 0.0722;
}
