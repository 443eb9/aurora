use std::{fs::File, path::Path};

use dyn_clone::DynClone;
use glam::{Mat4, Vec4};
use png::{Decoder, OutputInfo, Transformations};
use uuid::Uuid;
use wgpu::{
    util::{DeviceExt, TextureDataOrder},
    BindGroupLayout, BufferUsages, Extent3d, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages,
};

use crate::{
    render::{
        resource::{DynamicGpuBuffer, GpuCamera, GpuDirectionalLight, Vertex},
        scene::{GpuAssets, GpuScene},
        ShaderData, Transferable,
    },
    scene::{
        entity::{Camera, DirectionalLight},
        SceneObject,
    },
    WgpuRenderer,
};

impl Transferable for Camera {
    type GpuRepr = GpuCamera;

    fn transfer(&self, _renderer: &WgpuRenderer) -> Self::GpuRepr {
        Self::GpuRepr {
            view: Mat4::from_quat(self.transform.rotation)
                * Mat4::from_translation(self.transform.translation),
            proj: self.projection.compute_matrix(),
        }
    }
}

impl Transferable for DirectionalLight {
    type GpuRepr = GpuDirectionalLight;

    fn transfer(&self, _renderer: &WgpuRenderer) -> Self::GpuRepr {
        let linear_color = self.color.into_linear();

        Self::GpuRepr {
            position: self.transform.translation.extend(0.),
            direction: self.transform.local_neg_z().extend(0.),
            color: Vec4::new(linear_color.red, linear_color.green, linear_color.blue, 1.),
        }
    }
}

pub struct Texture {
    meta: OutputInfo,
    raw: Vec<u8>,
}

impl Transferable for Texture {
    type GpuRepr = wgpu::Texture;

    fn transfer(&self, renderer: &WgpuRenderer) -> Self::GpuRepr {
        renderer.device.create_texture_with_data(
            &renderer.queue,
            &TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: self.meta.width,
                    height: self.meta.height,
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
        )
    }
}

impl Texture {
    pub fn new(image: impl AsRef<Path>) -> std::io::Result<Self> {
        let mut decoder = Decoder::new(File::open(image)?);
        decoder.set_transformations(Transformations::normalize_to_color8());
        let mut reader = decoder.read_info()?;
        let mut raw = vec![0; reader.output_buffer_size()];

        Ok(Self {
            meta: reader.next_frame(&mut raw)?,
            raw,
        })
    }

    pub fn meta(&self) -> &OutputInfo {
        &self.meta
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
                .flat_map(|v| v.as_bytes())
                .map(|b| *b)
                .collect(),
        );
        b.write(&renderer.device, &renderer.queue);
        b
    }
}

pub trait Material: SceneObject + DynClone {
    fn bind_group_layout(&self, renderer: &WgpuRenderer) -> BindGroupLayout;
    /// The uuid here should be the individual uuid.
    fn create_bind_group(&self, renderer: &WgpuRenderer, assets: &mut GpuAssets, uuid: Uuid);
    fn prepare(&self, renderer: &WgpuRenderer, assets: &mut GpuAssets) -> u32;
}
