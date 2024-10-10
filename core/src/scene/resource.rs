use std::path::Path;

use dyn_clone::DynClone;
use glam::{Mat4, Vec2, Vec3};
use image::ImageResult;
use uuid::Uuid;
use wgpu::{
    util::{DeviceExt, TextureDataOrder},
    BufferUsages, Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages,
};

use crate::{
    render::{
        resource::{
            DynamicGpuBuffer, GpuCamera, GpuDirectionalLight, GpuPointLight, GpuSpotLight, Vertex,
        },
        scene::GpuAssets,
        Transferable,
    },
    scene::{
        entity::{
            Camera, DirectionalLight, Light, OrthographicProjection, PointLight, SpotLight,
            Transform,
        },
        SceneObject,
    },
    util::{self, cube::CUBE_MAP_FACES, ext::RgbToVec3},
    WgpuRenderer,
};

impl Transferable for Camera {
    type GpuRepr = GpuCamera;

    fn transfer(&self, _renderer: &WgpuRenderer) -> Self::GpuRepr {
        Self::GpuRepr {
            position_ws: self.transform.translation,
            view: self.transform.compute_matrix().inverse(),
            proj: self.projection.compute_matrix(),
            exposure: self.exposure.ev100,
        }
    }
}

impl Transferable for DirectionalLight {
    type GpuRepr = GpuDirectionalLight;

    fn transfer(&self, _renderer: &WgpuRenderer) -> Self::GpuRepr {
        Self::GpuRepr {
            direction: self.transform.local_neg_z(),
            color: self.color.into_linear().to_vec3(),
            intensity: self.intensity,
        }
    }
}

impl Transferable for PointLight {
    type GpuRepr = GpuPointLight;

    fn transfer(&self, _renderer: &WgpuRenderer) -> Self::GpuRepr {
        Self::GpuRepr {
            position: self.transform.translation,
            color: self.color.into_linear().to_vec3(),
            intensity: self.intensity,
        }
    }
}

impl Transferable for SpotLight {
    type GpuRepr = GpuSpotLight;

    fn transfer(&self, _renderer: &WgpuRenderer) -> Self::GpuRepr {
        Self::GpuRepr {
            position: self.transform.translation,
            direction: self.transform.local_neg_z(),
            color: self.color.into_linear().to_vec3(),
            intensity: self.intensity,
            inner_angle: self.inner_angle,
            outer_angle: self.outer_angle,
        }
    }
}

impl Light {
    pub fn as_cameras(&self, real_camera: &Camera) -> Vec<GpuCamera> {
        match self {
            Light::Directional(l) => vec![GpuCamera {
                view: l
                    .transform
                    // .with_translation(real_camera.transform.translation)
                    .compute_matrix()
                    .inverse(),
                proj: OrthographicProjection::symmetric(32., 32., 10., -10.).compute_matrix(),
                // position_ws: real_camera.transform.translation,
                position_ws: l.transform.translation,
                exposure: 0.,
            }],
            Light::Point(l) => CUBE_MAP_FACES
                .into_iter()
                .map(|face| {
                    let trans = Transform::default()
                        .looking_at(face.target, face.up)
                        .with_translation(l.transform.translation);
                    GpuCamera {
                        view: trans.compute_matrix().inverse(),
                        proj: Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1., 0.1, 20.),
                        position_ws: trans.translation,
                        exposure: 0.,
                    }
                })
                .collect(),
            Light::Spot(l) => CUBE_MAP_FACES
                .into_iter()
                .map(|face| {
                    let trans = Transform::default()
                        .looking_at(face.target, face.up)
                        .with_translation(l.transform.translation);
                    GpuCamera {
                        view: trans.compute_matrix().inverse(),
                        proj: Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1., 0.1, 20.),
                        position_ws: trans.translation,
                        exposure: 0.,
                    }
                })
                .collect(),
        }
    }
}

pub struct Image {
    width: u32,
    height: u32,
    raw: Vec<u8>,
}

impl Image {
    pub fn from_path(path: impl AsRef<Path>) -> ImageResult<Self> {
        let img = image::open(path)?.into_rgba8();

        Ok(Self {
            width: img.width(),
            height: img.height(),
            raw: img.into_raw(),
        })
    }

    pub fn from_raw(data: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            raw: data,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

impl Transferable for Image {
    type GpuRepr = Texture;

    fn transfer(&self, renderer: &WgpuRenderer) -> Self::GpuRepr {
        renderer.device.create_texture_with_data(
            &renderer.queue,
            &TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                view_formats: &[TextureFormat::Rgba8UnormSrgb],
            },
            TextureDataOrder::LayerMajor,
            &self.raw,
        )
    }
}

#[derive(Debug, Default, Clone)]
pub struct Mesh {
    raw: Vec<Vertex>,
}

impl Mesh {
    pub fn from_obj(path: impl AsRef<Path>) -> Vec<Self> {
        let mut source = Vec::new();
        std::io::Read::read_to_end(&mut std::fs::File::open(path).unwrap(), &mut source).unwrap();
        let obj = obj::ObjData::load_buf(&source[..]).unwrap();

        let mut meshes = Vec::new();
        for object in obj.objects {
            let mut vertices = Vec::new();
            for group in object.groups {
                vertices.clear();
                for poly in group.polys {
                    for end_index in 2..poly.0.len() {
                        for &index in &[0, end_index - 1, end_index] {
                            let obj::IndexTuple(position_id, Some(texture_id), Some(normal_id)) =
                                poly.0[index]
                            else {
                                unreachable!()
                            };

                            vertices.push(Vertex {
                                position: obj.position[position_id].into(),
                                normal: obj.normal[normal_id].into(),
                                uv: Vec2::from(obj.texture[texture_id]),
                                tangent: Default::default(),
                            });
                        }
                    }
                }
            }
            assert_eq!(vertices.len() % 3, 0, "Invalid mesh.");
            let mut mesh = Self { raw: vertices };
            mesh.recalculate_tangent();
            meshes.push(mesh);
        }

        meshes
    }

    pub fn vertices_count(&self) -> u32 {
        self.raw.len() as u32
    }

    pub fn recalculate_tangent(&mut self) {
        let mut tangents = vec![Vec3::default(); self.raw.len()];
        let mut bitangents = vec![Vec3::default(); self.raw.len()];

        for i_tri in 0..self.raw.len() / 3 {
            let i0 = i_tri * 3;
            let i1 = i0 + 1;
            let i2 = i1 + 1;

            let v0 = &self.raw[i0];
            let v1 = &self.raw[i1];
            let v2 = &self.raw[i2];

            let e1 = v1.position - v0.position;
            let e2 = v2.position - v0.position;

            let x1 = v1.uv.x - v0.uv.x;
            let x2 = v2.uv.x - v0.uv.x;

            let y1 = v1.uv.y - v0.uv.y;
            let y2 = v2.uv.y - v0.uv.y;

            let r = 1. / (x1 * y2 - x2 * y1);
            let t = (e1 * y2 - e2 * y1) * r;
            let b = (e2 * x1 - e1 * x2) * r;

            tangents[i0] += t;
            tangents[i1] += t;
            tangents[i2] += t;

            bitangents[i0] += b;
            bitangents[i1] += b;
            bitangents[i2] += b;
        }

        for i_vert in 0..self.raw.len() {
            let t = tangents[i_vert];
            let b = bitangents[i_vert];
            let n = self.raw[i_vert].normal;
            let sign = {
                if t.cross(b).dot(n) > 0. {
                    1.
                } else {
                    -1.
                }
            };

            self.raw[i_vert].tangent = n.reject_from(t).extend(sign);
        }
    }
}

impl Transferable for Mesh {
    type GpuRepr = DynamicGpuBuffer;

    fn transfer(&self, renderer: &WgpuRenderer) -> Self::GpuRepr {
        let mut b = DynamicGpuBuffer::new(BufferUsages::VERTEX);
        b.set(
            self.raw
                .iter()
                .flat_map(|v| util::struct_to_bytes(v))
                .map(|b| *b)
                .collect(),
        );
        b.write(&renderer.device, &renderer.queue);
        b
    }
}

pub trait Material: SceneObject + DynClone {
    fn create_layout(&self, renderer: &WgpuRenderer, assets: &mut GpuAssets);
    /// The uuid here should be the individual uuid.
    fn create_bind_group(&self, renderer: &WgpuRenderer, assets: &mut GpuAssets, uuid: Uuid);
    fn prepare(&self, renderer: &WgpuRenderer, assets: &mut GpuAssets) -> u32;
}
