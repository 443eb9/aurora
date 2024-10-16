use std::collections::HashMap;

use aurora_core::render::helper::{
    CameraProjection, OrthographicProjection, PerspectiveProjection,
};
use glam::{Mat4, Vec3, Vec4Swizzles};
use naga_oil::compose::{ComposableModuleDescriptor, Composer, ShaderDefValue};

pub fn add_shader_module(
    composer: &mut Composer,
    shader: &str,
    shader_defs: HashMap<String, ShaderDefValue>,
) {
    composer
        .add_composable_module(ComposableModuleDescriptor {
            source: shader,
            shader_defs,
            ..Default::default()
        })
        .unwrap();
}

#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

pub fn frustum_slice(proj: CameraProjection, count: u32) -> Vec<CameraProjection> {
    match proj {
        CameraProjection::Perspective(proj) => {
            let r = (proj.far / proj.near).powf(1. / count as f32);
            let mut near = proj.near;

            (0..count)
                .map(|_| {
                    let far = near * r;
                    let p =
                        CameraProjection::Perspective(PerspectiveProjection { near, far, ..proj });
                    near = far;
                    p
                })
                .collect()
        }
        CameraProjection::Orthographic(proj) => {
            let r = (proj.far / proj.near).powf(1. / count as f32);
            let mut near = proj.near;

            (0..count)
                .map(|_| {
                    let far = near * r;
                    let p = CameraProjection::Orthographic(OrthographicProjection {
                        near,
                        far,
                        ..proj
                    });
                    near = far;
                    p
                })
                .collect()
        }
    }
}

pub fn calculate_frustum_corners(view_proj: Mat4) -> [Vec3; 8] {
    let mut corners = [
        // Near Plane
        Vec3::new(1., 1., 0.),
        Vec3::new(-1., 1., 0.),
        Vec3::new(1., -1., 0.),
        Vec3::new(-1., -1., 0.),
        // Far Plane
        Vec3::new(1., 1., 1.),
        Vec3::new(-1., 1., 1.),
        Vec3::new(1., -1., 1.),
        Vec3::new(-1., -1., 1.),
    ];

    let mat = view_proj.inverse();
    corners.iter_mut().for_each(|c| {
        let clip = mat * c.extend(1.);
        *c = clip.xyz() / clip.w;
    });

    corners
}
