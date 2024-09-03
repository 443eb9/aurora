#define_import_path aurora::pbr::pbr_function
#import aurora::{
    common_binding::camera,
    math::PI,
    pbr::{
        pbr_binding::{tex_base_color, tex_normal, tex_sampler},
        pbr_type::{BrdfSurfaceLit, BrdfSurfaceUnlit, PbrMaterial, PbrVertexOutput}
    }
}

// Construct a BrdfSurface WITHOUT light related info.
fn construct_surface_unlit(
    position: vec3f,
    normal: vec3f,
    uv: vec2f,
    material: PbrMaterial,
) -> BrdfSurfaceUnlit {
    var surface: BrdfSurfaceUnlit;

    surface.roughness = material.roughness * material.roughness;
    surface.metallic = saturate(material.metallic);
    surface.base_color = (1. - surface.metallic) * material.base_color * textureSample(tex_base_color, tex_sampler, uv).rgb;
    
    surface.normal = normal;
    surface.view = normalize(camera.position - position);

    surface.f_normal = mix(vec3f(0.16 * material.reflectance * material.reflectance), surface.base_color, surface.metallic);

    surface.NdotV = saturate(dot(surface.normal, surface.view));

    return surface;
}

// Construct a BrdfSurface WITH light related info.
fn construct_surface_lit(light: vec3f, unlit: ptr<function, BrdfSurfaceUnlit>) -> BrdfSurfaceLit {
    var surface: BrdfSurfaceLit;

    surface.light = light;
    surface.half = normalize(surface.light + (*unlit).view);
    
    surface.NdotL = saturate(dot((*unlit).normal, surface.light));
    surface.NdotH = saturate(dot((*unlit).normal, surface.half));
    surface.HdotL = saturate(dot(surface.half, surface.light));

    return surface;
}

fn unpack_normal(normal_os: vec3f, tangent_os: vec4f, uv: vec2f) -> vec3f {
    let bitangent_os = cross(normal_os, tangent_os.xyz) * tangent_os.w;
    let ttw = mat3x3f(tangent_os.xyz, bitangent_os, normal_os);
    // TODO: Why 1. - normal_ts?
    return ttw * (1. - textureSample(tex_normal, tex_sampler, uv).xyz);
}

// GGX NDF
fn D_GGX(unlit: ptr<function, BrdfSurfaceUnlit>, lit: ptr<function, BrdfSurfaceLit>) -> f32 {
    let r2 = (*unlit).roughness * (*unlit).roughness;
    let den = 1. + (*lit).NdotH * (*lit).NdotH * (r2 - 1.);
    return r2 / (PI * den * den);
}

// Fresnel Reflectance
// Schlick approximation
fn F_Schlick(HdotL: f32, f_normal: vec3f) -> vec3f {
    return f_normal + (1. - f_normal) * pow(1. - HdotL, 5.);
}

// Simplified by Lagarde
// Notice this has already combined the denominator of specular BRDF.
fn G2_HeightCorrelated(unlit: ptr<function, BrdfSurfaceUnlit>, lit: ptr<function, BrdfSurfaceLit>) -> f32 {
    let r2 = (*unlit).roughness * (*unlit).roughness;
    let NdotL = (*lit).NdotL;
    let NdotV = (*unlit).NdotV;
    let l = NdotV * sqrt(r2 + NdotL * (NdotL - r2 * NdotL));
    let v = NdotL * sqrt(r2 + NdotV * (NdotV - r2 * NdotV));
    return 0.5 / (l + v);
}

fn FD_Lambert(unlit: ptr<function, BrdfSurfaceUnlit>, lit: ptr<function, BrdfSurfaceLit>) -> vec3f {
    return (1. - F_Schlick((*lit).HdotL, (*unlit).f_normal)) / PI;
}

fn FD_Burley(unlit: ptr<function, BrdfSurfaceUnlit>, lit: ptr<function, BrdfSurfaceLit>) -> vec3f {
    let f = 0.5 + 2. * (*unlit).roughness * (*lit).HdotL * (*lit).HdotL;
    let l = F_Schlick((*lit).NdotL, f);
    let v = F_Schlick((*unlit).NdotV, f);
    return l * v / PI;
}

fn apply_exposure(scene: vec3f) -> vec3f {
    return scene / (pow(2., camera.exposure) * 1.2);
}

fn apply_lighting(
    direction: vec3f,
    intensity: f32,
    color: vec3f,
    unlit: ptr<function, BrdfSurfaceUnlit>
) -> vec3f {
    var lit = construct_surface_lit(direction, unlit);

#ifdef GGX
    let D = D_GGX(unlit, &lit);
    let G = G2_HeightCorrelated(unlit, &lit);
#else
    let D = 0.;
    let G = 0.;
#endif

#ifdef LAMBERT
    let FD = FD_Lambert(unlit, &lit);
#else ifdef BURLEY
    let FD = FD_Burley(unlit, &lit);
#else
    let FD = vec3f(0.);
#endif

    let F = F_Schlick(lit.HdotL, (*unlit).f_normal);

    let f_spec = D * G * F * PI;
    let f_diff = FD * PI;

    return lit.NdotL * intensity * (f_spec + f_diff) * color;
}
