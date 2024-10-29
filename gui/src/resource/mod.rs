use std::rc::Rc;

use aurora_chest::{import::mesh_from_obj, material::PbrMaterial};
use aurora_core::{
    render::{
        helper::Transform,
        mesh::{Image, Mesh, StaticMesh},
        resource::{GpuDirectionalLight, GpuPointLight, GpuSpotLight},
        scene::{GpuScene, MaterialInstanceId, MeshInstanceId, TextureId},
    },
    WgpuRenderer,
};
use glam::{EulerRot, Mat4, Quat, Vec3};
use palette::Srgb;
use uuid::Uuid;

pub fn load_primitives(renderer: &WgpuRenderer) -> GpuScene {
    let mut scene = GpuScene::default();

    let uv_checker = TextureId(Uuid::new_v4());
    scene.assets.textures.insert(
        uv_checker,
        Image::from_path("gui/assets/uv_checker.png")
            .unwrap()
            .to_texture(&renderer.device, &renderer.queue),
    );
    let normal_map = TextureId(Uuid::new_v4());
    scene.assets.textures.insert(
        normal_map,
        Image::from_path("gui/assets/sergun-kuyucu-medieval-blocks-normal.png")
            .unwrap()
            .to_texture(&renderer.device, &renderer.queue),
    );

    let meshes =
    // mesh_from_obj("gui/assets/large_primitives_offseted.obj")
        // mesh_from_obj("gui/assets/Room.obj")
        // mesh_from_obj("gui/assets/cube.obj")
        mesh_from_obj("gui/assets/large_scene_cascade_test.obj")
        .into_iter()
        .map(|m| {
            let instance_id = MeshInstanceId(Uuid::new_v4());
            scene.assets.meshes.insert(
                instance_id,
                m
            );
            instance_id
        })
        .collect::<Vec<_>>();
    let static_meshes = meshes.into_iter().enumerate().map(|(_, mesh)| {
        let instance_id = MaterialInstanceId(Uuid::new_v4());
        scene.original.materials.insert(
            instance_id,
            Rc::new(PbrMaterial {
                base_color: Srgb::new(1., 1., 1.),
                tex_base_color: Some(uv_checker),
                tex_normal: Some(normal_map),
                reflectance: 0.15,
                roughness: 0.3,
                metallic: 0.,
            }),
        );

        StaticMesh {
            mesh,
            material: instance_id,
        }
    });
    scene.static_meshes.extend(static_meshes);

    scene.original.dir_lights.insert(
        Uuid::new_v4(),
        GpuDirectionalLight {
            // direction: Transform {
            //     rotation: Quat::from_euler(EulerRot::XYZ, -0.5, -0.2, 0.),
            //     ..Default::default()
            // }
            // .local_z(),
            direction: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4) * Vec3::Z,
            // direction: Vec3::Z,
            color: Srgb::new(1., 1., 1.).into_linear().into_components().into(),
            intensity: 2000.,
            radius: 1.,
        },
    );
    // scene.original.point_lights.insert(
    //     Uuid::new_v4(),
    //     GpuPointLight {
    //         // transform: Transform {
    //         //     translation: Vec3 {
    //         //         x: -2.,
    //         //         y: 1.5,
    //         //         z: 0.,
    //         //     },
    //         //     ..Default::default()
    //         // },
    //         position: Vec3::ZERO,
    //         color: Srgb::new(0.2, 0.5, 0.8)
    //             .into_linear()
    //             .into_components()
    //             .into(),
    //         intensity: 10000.,
    //         radius: 2.,
    //     },
    // );
    // scene.original.spot_lights.insert(
    //     Uuid::new_v4(),
    //     GpuSpotLight {
    //         // transform: Transform {
    //         //     // translation: Vec3 {
    //         //     //     x: 2.,
    //         //     //     y: 2.,
    //         //     //     z: -2.,
    //         //     // },
    //         //     translation: Vec3::ZERO,
    //         //     rotation: ,
    //         //     ..Default::default()
    //         // },
    //         position: Vec3::ZERO,
    //         direction: Transform::default()
    //             .with_rotation(Quat::from_axis_angle(Vec3::X, std::f32::consts::FRAC_PI_3))
    //             .local_neg_z(),
    //         color: Srgb::new(0., 1., 0.).into_linear().into_components().into(),
    //         intensity: 10000.,
    //         radius: 2.,
    //         inner_angle: std::f32::consts::FRAC_PI_6 * 0.75,
    //         outer_angle: std::f32::consts::FRAC_PI_4 * 0.75,
    //     },
    // );

    scene
}
