use aurora_chest::material::PbrMaterial;
use aurora_core::scene::{
    entity::{DirectionalLight, Light, PointLight, SpotLight, StaticMesh, Transform},
    resource::{Image, Mesh},
    Scene,
};
use glam::{Mat4, Quat, Vec3};
use palette::Srgb;

pub fn load_primitives() -> Scene {
    let mut scene = Scene::default();

    let uv_checker = scene.insert_object(Image::from_path("gui/assets/uv_checker.png").unwrap());
    let normal_map = scene.insert_object(
        Image::from_path("gui/assets/sergun-kuyucu-medieval-blocks-normal.png").unwrap(),
    );

    let meshes = 
    // Mesh::from_obj("gui/assets/large_primitives.obj")
    Mesh::from_obj("gui/assets/Room.obj")
        .into_iter()
        .map(|m| scene.insert_object(m))
        .collect::<Vec<_>>();
    let static_meshes = meshes
        .into_iter()
        .enumerate()
        .map(|(_, mesh)| StaticMesh {
            mesh,
            material: scene.insert_object(PbrMaterial {
                base_color: Srgb::new(1., 1., 1.),
                tex_base_color: Some(uv_checker),
                tex_normal: Some(normal_map),
                reflectance: 0.15,
                roughness: 0.3,
                metallic: 0.,
            }),
        })
        .collect::<Vec<_>>();
    static_meshes.into_iter().for_each(|sm| {
        scene.insert_object(sm);
    });

    // scene.insert_object(Light::Directional(DirectionalLight {
    //     transform: Transform {
    //         rotation: Quat::from_mat4(&Mat4::look_at_lh(Vec3::ZERO, Vec3::NEG_ONE, Vec3::Y)),
    //         ..Default::default()
    //     },
    //     color: Srgb::new(1., 1., 1.),
    //     intensity: 2000.,
    // }));
    scene.insert_object(Light::Point(PointLight {
        // transform: Transform {
        //     translation: Vec3 {
        //         x: -2.,
        //         y: 1.5,
        //         z: 0.,
        //     },
        //     ..Default::default()
        // },
        transform: Transform::default(),
        color: Srgb::new(0.2, 0.5, 0.8),
        intensity: 10000.,
    }));
    scene.insert_object(Light::Spot(SpotLight {
        transform: Transform {
            // translation: Vec3 {
            //     x: 2.,
            //     y: 2.,
            //     z: -2.,
            // },
            translation: Vec3::ZERO,
            rotation: Quat::from_axis_angle(Vec3::X, std::f32::consts::FRAC_PI_3),
            ..Default::default()
        },
        color: Srgb::new(0., 1., 0.),
        intensity: 10000.,
        inner_angle: std::f32::consts::FRAC_PI_6 * 0.75,
        outer_angle: std::f32::consts::FRAC_PI_4 * 0.75,
    }));

    scene
}
