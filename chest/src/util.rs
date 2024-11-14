use std::collections::HashMap;

use aurora_core::render::helper::{
    CameraProjection, OrthographicProjection, PerspectiveProjection,
};
use glam::{Mat4, Vec3, Vec4Swizzles};
use naga_oil::compose::{
    ComposableModuleDescriptor, Composer, ComposerError, NagaModuleDescriptor, ShaderDefValue,
};
use wgpu::naga::Module;

pub fn build_shader<const N: usize>(
    deps: [&str; N],
    main: &str,
    defs: impl Into<HashMap<String, ShaderDefValue>>,
) -> Result<Module, ComposerError> {
    let defs = defs.into();
    let mut composer = Composer::default();
    for dep in deps {
        composer.add_composable_module(ComposableModuleDescriptor {
            source: &dep,
            shader_defs: defs.clone(),
            ..Default::default()
        })?;
    }
    composer.make_naga_module(NagaModuleDescriptor {
        source: &main,
        shader_defs: defs.clone(),
        ..Default::default()
    })
}

#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

pub fn frustum_slice(proj: CameraProjection, count: u32, lambda: f32) -> Vec<CameraProjection> {
    match proj {
        CameraProjection::Perspective(proj) => {
            let r = (proj.far / proj.near).powf(1. / count as f32);
            let d = proj.far - proj.near;
            let mut near = proj.near;

            (0..count)
                .map(|x| {
                    let x = x as f32;
                    let d_log = proj.near * r.powf(x);
                    let d_uni = proj.near + d / count as f32 * (x + 1.);
                    let d_slice = lambda * d_log + (1. - lambda) * d_uni;
                    near += d_slice;

                    CameraProjection::Perspective(PerspectiveProjection {
                        near: near - d_slice,
                        far: near,
                        ..proj
                    })
                })
                .collect()
        }
        CameraProjection::Orthographic(proj) => {
            let r = (proj.far / proj.near).powf(1. / count as f32);
            let d = proj.far - proj.near;
            let mut near = proj.near;

            (0..count)
                .map(|x| {
                    let x = x as f32;
                    let d_log = proj.near * r.powf(x);
                    let d_uni = proj.near + d / count as f32 * (x + 1.);
                    let d_slice = lambda * d_log + (1. - lambda) * d_uni;
                    near += d_slice;

                    CameraProjection::Orthographic(OrthographicProjection {
                        near: near - d_slice,
                        far: near,
                        ..proj
                    })
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
