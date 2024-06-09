use std::path::Path;

use dyn_clone::DynClone;
use image::ImageResult;
use uuid::Uuid;
use wgpu::{
    util::{DeviceExt, TextureDataOrder},
    BufferUsages, Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages,
};

use crate::{
    render::{
        resource::{DynamicGpuBuffer, GpuCamera, GpuDirectionalLight, Vertex},
        scene::GpuAssets,
        Transferable,
    },
    scene::{
        entity::{Camera, DirectionalLight},
        SceneObject,
    },
    util::{self, ext::RgbToVec3},
    WgpuRenderer,
};

impl Transferable for Camera {
    type GpuRepr = GpuCamera;

    fn transfer(&self, _renderer: &WgpuRenderer) -> Self::GpuRepr {
        Self::GpuRepr {
            position_ws: self.transform.translation,
            view: self.transform.compute_matrix().inverse(),
            proj: self.projection.compute_matrix(),
        }
    }
}

impl Transferable for DirectionalLight {
    type GpuRepr = GpuDirectionalLight;

    fn transfer(&self, _renderer: &WgpuRenderer) -> Self::GpuRepr {
        Self::GpuRepr {
            position: self.transform.translation,
            direction: self.transform.local_neg_z(),
            color: self.color.into_linear().to_vec3(),
            illuminance: self.illuminance,
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
        let img = image::open(path)?;

        Ok(Self {
            width: img.width(),
            height: img.height(),
            raw: img.into_bytes(),
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
                                uv: glam::Vec2::from(obj.texture[texture_id]).extend(0.),
                            });
                        }
                    }
                }
            }
            meshes.push(Self { raw: vertices });
        }

        meshes
    }

    pub fn vertex_count(&self) -> u32 {
        self.raw.len() as u32
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
