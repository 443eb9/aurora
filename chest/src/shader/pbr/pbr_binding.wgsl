#define_import_path aurora::pbr::pbr_binding
#import aurora::{
    common_type::{Camera, DirectionalLight, PointLight, SpotLight},
    pbr::pbr_type::PbrMaterial
}

@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var<storage, read> dir_lights: array<DirectionalLight>;
@group(1) @binding(1) var<storage, read> point_lights: array<PointLight>;
@group(1) @binding(2) var<storage, read> spot_lights: array<SpotLight>;
@group(1) @binding(3) var<storage, read> area_light_vertices: array<vec3f>;

@group(2) @binding(0) var<uniform> material: PbrMaterial;
@group(2) @binding(1) var tex_base_color: texture_2d<f32>;
@group(2) @binding(2) var tex_normal: texture_2d<f32>;
@group(2) @binding(3) var tex_sampler: sampler;
