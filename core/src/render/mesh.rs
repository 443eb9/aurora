use std::{any::TypeId, path::Path};

use dyn_clone::DynClone;
use glam::{Vec2, Vec3};
use image::ImageResult;
use wgpu::{
    util::{DeviceExt, TextureDataOrder},
    BufferUsages, Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages,
};

use crate::{
    render::{
        resource::{DynamicGpuBuffer, Vertex},
        scene::{GpuAssets, MaterialInstanceId, MaterialTypeId, MeshInstanceId},
    },
    util::{self, ext::TypeIdAsUuid},
    WgpuRenderer,
};

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

    pub fn to_texture(&self, renderer: &WgpuRenderer) -> Texture {
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

    pub fn to_vertex_buffer(&self, renderer: &WgpuRenderer) -> DynamicGpuBuffer {
        let mut b = DynamicGpuBuffer::new(BufferUsages::VERTEX);
        b.set(
            self.raw
                .iter()
                .flat_map(|v| util::struct_to_bytes(v))
                .map(|b| *b)
                .collect(),
        );
        b.write::<Vertex>(&renderer.device, &renderer.queue);
        b
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StaticMesh {
    pub mesh: MeshInstanceId,
    pub material: MaterialInstanceId,
}

pub trait Material: DynClone + 'static {
    fn create_bind_group(
        &self,
        renderer: &WgpuRenderer,
        assets: &mut GpuAssets,
        material: MaterialInstanceId,
    );
    fn prepare(&self, renderer: &WgpuRenderer, assets: &mut GpuAssets) -> u32;

    #[inline]
    fn id(&self) -> MaterialTypeId {
        MaterialTypeId(TypeId::of::<Self>().to_uuid())
    }
}

pub trait CreateBindGroupLayout {
    fn create_layout(renderer: &WgpuRenderer, assets: &mut GpuAssets);
}
