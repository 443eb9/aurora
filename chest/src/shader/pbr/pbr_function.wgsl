#define_import_path aurora::pbr::pbr_function
#import aurora::{
    math::PI,
    pbr::{
        pbr_binding::{camera, tex_base_color, tex_sampler},
        pbr_type::{BrdfSurfaceLit, BrdfSurfaceUnlit, PbrMaterial, PbrVertexOutput}
    }
}

// Construct a BrdfSurface WITHOUT light related info.
fn construct_surface_unlit(vert: PbrVertexOutput, material: PbrMaterial, uv: vec2f) -> BrdfSurfaceUnlit {
    var surface: BrdfSurfaceUnlit;

    surface.base_color = material.base_color * textureSample(tex_base_color, tex_sampler, uv).rgb;
    surface.roughness = material.roughness * material.roughness;
    surface.metallic = material.metallic;

    surface.normal = vert.normal_ws;
    surface.view = normalize(camera.position_ws - vert.position_ws);

    surface.f_normal = material.f_normal;
    surface.f_glance = material.f_glance;

    surface.NdotV = abs(dot(surface.normal, surface.view)) + 1e-5;

    return surface;
}

// Construct a BrdfSurface WITH light related info.
fn construct_surface_lit(light: vec3f, unlit: BrdfSurfaceUnlit) -> BrdfSurfaceLit {
    var surface: BrdfSurfaceLit;

    surface.light = light;
    surface.half = normalize(surface.light + unlit.view);
    
    surface.NdotL = saturate(dot(unlit.normal, surface.light));
    surface.NdotH = saturate(dot(unlit.normal, surface.half));
    surface.HdotL = saturate(dot(surface.half, surface.light));

    return surface;
}

// GGX NDF
fn D_GGX(roughness: f32, NdotH: f32) -> f32 {
    let r2 = roughness * roughness;
    let den = 1. + NdotH * NdotH * (r2 - 1.);
    return r2 / (PI * den * den);
}

// Fresnel Reflectance
// Schlick approximation
fn F_Schlick(HdotL: f32, f_normal: vec3f) -> vec3f {
    return f_normal + (vec3f(1.) - f_normal) * vec3f(pow(1. - HdotL, 5.));
}

// Simplified by Lagarde
// Notice this has already combined the denominator of specular BRDF.
fn G2_HeightCorrelated(roughness: f32, NdotL: f32, NdotV: f32) -> f32 {
    let r2 = roughness * roughness;
    let l = NdotV * sqrt(r2 + NdotL * (NdotL - r2 * NdotL));
    let v = NdotL * sqrt(r2 + NdotV * (NdotV - r2 * NdotV));
    return 0.5 / (l + v);
}

fn FD_Lambert(rho_ss: vec3f) -> vec3f {
    return rho_ss / PI;
}
