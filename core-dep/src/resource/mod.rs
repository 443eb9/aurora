use std::{fs::File, path::Path};

use aurora_derive::ShaderData;

use bytemuck::{Pod, Zeroable};

use glam::{UVec2, Vec3};

use png::{Decoder, Transformations};
use wgpu::{
    util::{DeviceExt, TextureDataOrder},
    Device, Extent3d, Queue, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

pub mod buffer;
pub mod material;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResRef(pub(crate) u128);

impl ResRef {
    pub const fn new(id: u128) -> Self {
        Self(id)
    }
}

#[derive(ShaderData, Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub material: ResRef,
}

impl Mesh {
    pub fn from_obj(path: impl AsRef<Path>, material: ResRef) -> Self {
        let mut source = Vec::new();
        std::io::Read::read_to_end(&mut std::fs::File::open(path).unwrap(), &mut source).unwrap();
        let obj = obj::ObjData::load_buf(&source[..]).unwrap();

        let mut vertices = Vec::new();
        for object in obj.objects {
            for group in object.groups {
                vertices.clear();
                for poly in group.polys {
                    for end_index in 2..poly.0.len() {
                        for &index in &[0, end_index - 1, end_index] {
                            let obj::IndexTuple(position_id, Some(_texture_id), Some(normal_id)) =
                                poly.0[index]
                            else {
                                unreachable!()
                            };

                            vertices.push(Vertex {
                                position: obj.position[position_id].into(),
                                normal: obj.normal[normal_id].into(),
                            });
                        }
                    }
                }
            }
        }

        Self { vertices, material }
    }
}

pub struct Texture {
    pub dim: UVec2,
    pub raw: Vec<u8>,
}

impl Texture {
    pub fn new(image: impl AsRef<Path>) -> std::io::Result<Self> {
        let mut decoder = Decoder::new(File::open(image)?);
        decoder.set_transformations(Transformations::normalize_to_color8());
        let mut reader = decoder.read_info()?;
        let mut raw = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut raw)?;

        Ok(Self {
            dim: UVec2 {
                x: info.width,
                y: info.height,
            },
            raw,
        })
    }

    pub fn clone_to_gpu(&self, device: &Device, queue: &Queue) -> GpuTexture {
        GpuTexture {
            texture: device.create_texture_with_data(
                queue,
                &TextureDescriptor {
                    label: None,
                    size: Extent3d {
                        width: self.dim.x,
                        height: self.dim.y,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8Unorm,
                    usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                    view_formats: &[TextureFormat::Rgba8Unorm],
                },
                TextureDataOrder::LayerMajor,
                &self.raw,
            ),
        }
    }
}

pub struct GpuTexture {
    pub texture: wgpu::Texture,
}
