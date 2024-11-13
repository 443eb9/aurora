use std::{any::TypeId, collections::BTreeMap};

use dyn_clone::DynClone;
use glam::{IVec2, IVec3, IVec4, Mat3, Mat4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4};
use log::warn;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer, BufferUsages, Device, IndexFormat, VertexAttribute, VertexFormat,
};

use crate::{
    render::scene::{GpuAssets, MaterialInstanceId, MaterialTypeId, MeshInstanceId},
    util::ext::TypeIdAsUuid,
};

#[derive(Clone, PartialEq, Eq)]
pub struct MeshVertexAttributeId {
    pub id: usize,
    pub name: &'static str,
    pub format: VertexFormat,
}

impl PartialOrd for MeshVertexAttributeId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.id.cmp(&other.id))
    }
}

impl Ord for MeshVertexAttributeId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl MeshVertexAttributeId {
    pub const fn new(id: usize, name: &'static str, format: VertexFormat) -> Self {
        Self { id, name, format }
    }
}

#[derive(Clone)]
pub enum MeshVertexAttributeData {
    Sint32(Vec<i32>),
    Uint32(Vec<u32>),
    Float32(Vec<f32>),
    Sint32x2(Vec<IVec2>),
    Uint23x2(Vec<UVec2>),
    Float32x2(Vec<Vec2>),
    Sint32x3(Vec<IVec3>),
    Uint23x3(Vec<UVec3>),
    Float32x3(Vec<Vec3>),
    Sint32x4(Vec<IVec4>),
    Uint23x4(Vec<UVec4>),
    Float32x4(Vec<Vec4>),
}

impl MeshVertexAttributeData {
    pub fn len(&self) -> usize {
        match self {
            MeshVertexAttributeData::Sint32(vec) => vec.len(),
            MeshVertexAttributeData::Uint32(vec) => vec.len(),
            MeshVertexAttributeData::Float32(vec) => vec.len(),
            MeshVertexAttributeData::Sint32x2(vec) => vec.len(),
            MeshVertexAttributeData::Uint23x2(vec) => vec.len(),
            MeshVertexAttributeData::Float32x2(vec) => vec.len(),
            MeshVertexAttributeData::Sint32x3(vec) => vec.len(),
            MeshVertexAttributeData::Uint23x3(vec) => vec.len(),
            MeshVertexAttributeData::Float32x3(vec) => vec.len(),
            MeshVertexAttributeData::Sint32x4(vec) => vec.len(),
            MeshVertexAttributeData::Uint23x4(vec) => vec.len(),
            MeshVertexAttributeData::Float32x4(vec) => vec.len(),
        }
    }

    pub fn format(&self) -> VertexFormat {
        match self {
            MeshVertexAttributeData::Sint32(_) => VertexFormat::Sint32,
            MeshVertexAttributeData::Uint32(_) => VertexFormat::Uint32,
            MeshVertexAttributeData::Float32(_) => VertexFormat::Float32,
            MeshVertexAttributeData::Sint32x2(_) => VertexFormat::Sint32x2,
            MeshVertexAttributeData::Uint23x2(_) => VertexFormat::Uint32x2,
            MeshVertexAttributeData::Float32x2(_) => VertexFormat::Float32x2,
            MeshVertexAttributeData::Sint32x3(_) => VertexFormat::Sint32x3,
            MeshVertexAttributeData::Uint23x3(_) => VertexFormat::Uint32x3,
            MeshVertexAttributeData::Float32x3(_) => VertexFormat::Float32x3,
            MeshVertexAttributeData::Sint32x4(_) => VertexFormat::Sint32x4,
            MeshVertexAttributeData::Uint23x4(_) => VertexFormat::Uint32x4,
            MeshVertexAttributeData::Float32x4(_) => VertexFormat::Float32x4,
        }
    }

    pub fn cast_bytes(&self) -> &[u8] {
        match self {
            MeshVertexAttributeData::Sint32(vec) => bytemuck::cast_slice(vec),
            MeshVertexAttributeData::Uint32(vec) => bytemuck::cast_slice(vec),
            MeshVertexAttributeData::Float32(vec) => bytemuck::cast_slice(vec),
            MeshVertexAttributeData::Sint32x2(vec) => bytemuck::cast_slice(vec),
            MeshVertexAttributeData::Uint23x2(vec) => bytemuck::cast_slice(vec),
            MeshVertexAttributeData::Float32x2(vec) => bytemuck::cast_slice(vec),
            MeshVertexAttributeData::Sint32x3(vec) => bytemuck::cast_slice(vec),
            MeshVertexAttributeData::Uint23x3(vec) => bytemuck::cast_slice(vec),
            MeshVertexAttributeData::Float32x3(vec) => bytemuck::cast_slice(vec),
            MeshVertexAttributeData::Sint32x4(vec) => bytemuck::cast_slice(vec),
            MeshVertexAttributeData::Uint23x4(vec) => bytemuck::cast_slice(vec),
            MeshVertexAttributeData::Float32x4(vec) => bytemuck::cast_slice(vec),
        }
    }

    pub fn size(&self) -> u64 {
        self.format().size()
    }
}

pub struct GpuIndexBuffer {
    pub buffer: Buffer,
    pub count: u32,
    pub format: IndexFormat,
}

#[derive(Debug, Clone)]
pub enum MeshIndices {
    UInt16(Vec<u16>),
    UInt32(Vec<u32>),
}

#[derive(Default, Clone)]
pub struct Mesh {
    attributes: BTreeMap<MeshVertexAttributeId, MeshVertexAttributeData>,
    indices: Option<MeshIndices>,
}

impl Mesh {
    pub const POSITION_ATTR: MeshVertexAttributeId =
        MeshVertexAttributeId::new(0, "Position", VertexFormat::Float32x3);

    pub const NORMAL_ATTR: MeshVertexAttributeId =
        MeshVertexAttributeId::new(1, "Normal", VertexFormat::Float32x3);

    pub const TEX_COORDS_ATTR: MeshVertexAttributeId =
        MeshVertexAttributeId::new(2, "TexCoords", VertexFormat::Float32x2);

    pub const TANGENT_ATTR: MeshVertexAttributeId =
        MeshVertexAttributeId::new(3, "Tangent", VertexFormat::Float32x4);

    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_attribute(
        &mut self,
        id: MeshVertexAttributeId,
        data: MeshVertexAttributeData,
    ) -> &mut Self {
        self.attributes.insert(id, data);
        self
    }

    pub fn with_attribute(
        mut self,
        id: MeshVertexAttributeId,
        data: MeshVertexAttributeData,
    ) -> Self {
        self.attributes.insert(id, data);
        self
    }

    pub fn insert_indices(&mut self, indices: MeshIndices) -> &mut Self {
        self.indices = Some(indices);
        self
    }

    pub fn with_indices(mut self, indices: MeshIndices) -> Self {
        self.indices = Some(indices);
        self
    }

    pub fn recalculate_tangent(&mut self) {
        let vertices_count = self.vertices_count();
        let mut tangents = vec![Vec3::default(); vertices_count];
        let mut bitangents = vec![Vec3::default(); vertices_count];
        let mut result = vec![Vec4::default(); vertices_count];

        let (
            MeshVertexAttributeData::Float32x3(positions),
            MeshVertexAttributeData::Float32x3(normals),
            MeshVertexAttributeData::Float32x2(uvs),
        ) = (
            &self.attributes[&Self::POSITION_ATTR],
            &self.attributes[&Self::NORMAL_ATTR],
            &self.attributes[&Self::TEX_COORDS_ATTR],
        )
        else {
            unreachable!()
        };

        for i_tri in 0..vertices_count / 3 {
            let i0 = i_tri * 3;
            let i1 = i0 + 1;
            let i2 = i1 + 1;

            let p0 = positions[i0];
            let p1 = positions[i1];
            let p2 = positions[i2];

            let u0 = uvs[i0];
            let u1 = uvs[i1];
            let u2 = uvs[i2];

            let e1 = p1 - p0;
            let e2 = p2 - p0;

            let x1 = u1.x - u0.x;
            let x2 = u2.x - u0.x;

            let y1 = u1.y - u0.y;
            let y2 = u2.y - u0.y;

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

        for i_vert in 0..vertices_count {
            let t = tangents[i_vert];
            let b = bitangents[i_vert];
            let n = normals[i_vert];
            let sign = {
                if t.cross(b).dot(n) > 0. {
                    1.
                } else {
                    -1.
                }
            };

            result[i_vert] = n.reject_from(t).extend(sign);
        }

        self.attributes.insert(
            Self::TANGENT_ATTR,
            MeshVertexAttributeData::Float32x4(result),
        );
    }

    pub fn vertex_layout(&self) -> Vec<VertexAttribute> {
        let mut layout = Vec::with_capacity(self.attributes.len());
        let mut offset = 0;

        for (index, attr) in self.attributes.keys().enumerate() {
            layout.push(VertexAttribute {
                format: attr.format,
                offset,
                shader_location: index as u32,
            });
            offset += attr.format.size();
        }
        layout
    }

    pub fn vertices_count(&self) -> usize {
        let mut cnt = None;

        for data in self.attributes.values() {
            let count = data.len();
            if let Some(vertices) = cnt.as_mut() {
                if count != *vertices {
                    warn!("Stripping vertices: {} != {}", count, vertices);
                    *vertices = count.min(*vertices);
                }
            } else {
                cnt = Some(count);
            }
        }
        cnt.unwrap_or(0)
    }

    pub fn vertex_stride(&self) -> u64 {
        self.attributes
            .keys()
            .fold(0, |acc, a| acc + a.format.size())
    }

    pub fn vertex_buffer_data(&self) -> Vec<u8> {
        let vertices_count = self.vertices_count();
        let vertex_stride = self.vertex_stride() as usize;
        let mut buffer = vec![0; vertices_count * vertex_stride];

        let mut attr_offset = 0;
        for attr in self.attributes.values() {
            let attr_size = attr.size() as usize;
            let attr_bytes = attr.cast_bytes();

            for (vertex_index, data) in buffer.chunks_exact_mut(vertex_stride).enumerate() {
                let attr_base = vertex_index * attr_size;
                data[attr_offset..attr_offset + attr_size]
                    .copy_from_slice(&attr_bytes[attr_base..attr_base + attr_size]);
            }

            attr_offset += attr_size;
        }
        buffer
    }

    pub fn create_vertex_buffer(&self, device: &Device) -> Option<Buffer> {
        let data = self.vertex_buffer_data();
        if data.is_empty() {
            None
        } else {
            Some(device.create_buffer_init(&BufferInitDescriptor {
                label: Some("mesh_vertex_buffer"),
                contents: &data,
                usage: BufferUsages::VERTEX,
            }))
        }
    }

    pub fn create_index_buffer(&self, device: &Device) -> Option<GpuIndexBuffer> {
        self.indices.as_ref().map(|indices| {
            let (contents, format, count) = match indices {
                MeshIndices::UInt16(vec) => (
                    bytemuck::cast_slice(&vec),
                    IndexFormat::Uint16,
                    vec.len() as u32,
                ),
                MeshIndices::UInt32(vec) => (
                    bytemuck::cast_slice(&vec),
                    IndexFormat::Uint32,
                    vec.len() as u32,
                ),
            };
            GpuIndexBuffer {
                buffer: device.create_buffer_init(&BufferInitDescriptor {
                    label: Some("mesh_index_buffer"),
                    contents,
                    usage: BufferUsages::INDEX,
                }),
                count,
                format,
            }
        })
    }

    pub fn vertex_attributes(&self) -> Vec<VertexAttribute> {
        let mut attrs = Vec::with_capacity(self.attributes.len());
        let mut offset = 0;

        for (location, attr) in self.attributes.keys().enumerate() {
            attrs.push(VertexAttribute {
                format: attr.format,
                offset,
                shader_location: location as u32,
            });
            offset += attr.format.size();
        }

        attrs
    }

    pub fn assert_vertex(&self, attrs: &[VertexFormat]) {
        assert_eq!(attrs.len(), self.attributes.len());

        self.attributes
            .keys()
            .zip(attrs)
            .for_each(|(lhs, rhs)| assert_eq!(lhs.format, *rhs, "{} not matching.", lhs.name));
    }

    pub fn transform(&mut self, mat: Mat4) {
        if let Some(MeshVertexAttributeData::Float32x3(vertices)) =
            self.attributes.get_mut(&Self::POSITION_ATTR)
        {
            for v in vertices {
                *v = mat.transform_point3(*v);
            }
        }

        if let Some(MeshVertexAttributeData::Float32x3(normals)) =
            self.attributes.get_mut(&Self::NORMAL_ATTR)
        {
            let normal_mat = Mat3::from_cols(
                mat.x_axis.truncate().normalize(),
                mat.y_axis.truncate().normalize(),
                mat.z_axis.truncate().normalize(),
            );

            for n in normals {
                *n = normal_mat * *n;
            }
        }
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
        device: &Device,
        assets: &mut GpuAssets,
        material: MaterialInstanceId,
    );
    fn prepare(&self, device: &Device, assets: &mut GpuAssets) -> u32;

    #[inline]
    fn id(&self) -> MaterialTypeId {
        MaterialTypeId(TypeId::of::<Self>().to_uuid())
    }
}

pub trait CreateBindGroupLayout {
    fn create_layout(device: &Device, assets: &mut GpuAssets);
}
