use std::{collections::HashSet, f32::consts::FRAC_PI_4, path::Path, rc::Rc};

use glam::{Mat4, Quat, Vec2, Vec3, Vec4};
use gltf::{
    accessor::DataType,
    json::{Index, Node, Root},
    Gltf, Semantic,
};
use image::ImageFormat;
use palette::Srgb;
use thiserror::Error;
use uuid::Uuid;

use aurora_core::render::{
    helper::{
        Camera, CameraProjection, Exposure, OrthographicProjection, PerspectiveProjection,
        Transform,
    },
    mesh::{Mesh, MeshIndices, MeshVertexAttributeData, StaticMesh},
    resource::{GpuDirectionalLight, GpuPointLight, GpuSpotLight, Image},
    scene::{GpuScene, MaterialInstanceId, MeshInstanceId, TextureId},
};
use wgpu::{Device, Queue};

use crate::material::PbrMaterial;

pub enum BufferType {
    I8,
    U8,
    I16,
    U16,
    U32,
    F32,
}

#[derive(Error, Debug)]
pub enum GltfLoadError {
    #[error("{0}")]
    GltfParse(#[from] gltf::Error),
    #[error("{0} cameras found, expected 1.")]
    MultipleCameras(u32),
    #[error("Missing blob.")]
    MissingBlob,
    #[error["{0}"]]
    Base64DecodeError(#[from] base64::DecodeError),
    #[error("Buffer format unsupported.")]
    BufferFormatUnsupported,
    #[error("{0}")]
    IoError(#[from] std::io::Error),
}

pub type GltfLoadResult<T> = Result<T, GltfLoadError>;

struct DataUri<'a> {
    mime_type: &'a str,
    base64: bool,
    data: &'a str,
}

fn split_once(input: &str, delimiter: char) -> Option<(&str, &str)> {
    let mut iter = input.splitn(2, delimiter);
    Some((iter.next()?, iter.next()?))
}

impl<'a> DataUri<'a> {
    fn parse(uri: &'a str) -> Result<DataUri<'a>, ()> {
        let uri = uri.strip_prefix("data:").ok_or(())?;
        let (mime_type, data) = split_once(uri, ',').ok_or(())?;

        let (mime_type, base64) = match mime_type.strip_suffix(";base64") {
            Some(mime_type) => (mime_type, true),
            None => (mime_type, false),
        };

        Ok(DataUri {
            mime_type,
            base64,
            data,
        })
    }

    fn decode(&self) -> Result<Vec<u8>, base64::DecodeError> {
        if self.base64 {
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, self.data)
        } else {
            Ok(self.data.as_bytes().to_owned())
        }
    }
}

pub fn load_gltf(
    path: impl AsRef<Path>,
    device: &Device,
    queue: &Queue,
) -> GltfLoadResult<GpuScene> {
    let mut scene = GpuScene::default();
    let model = Gltf::open(path)?;
    let json = model.as_json();

    if model.cameras().len() > 1 {
        return Err(GltfLoadError::MultipleCameras(model.cameras().len() as u32));
    } else if model.cameras().len() == 0 {
        scene.original.camera = Camera {
            transform: Default::default(),
            projection: CameraProjection::Perspective(PerspectiveProjection {
                fov: FRAC_PI_4,
                aspect_ratio: 1.7777777777,
                near: 0.1,
                far: 200.,
            }),
            exposure: Default::default(),
        };
    }

    let mut linear_textures = HashSet::new();
    for mat in model.materials() {
        if let Some(tex) = mat.normal_texture() {
            linear_textures.insert(tex.texture().index());
        }
        if let Some(tex) = mat.occlusion_texture() {
            linear_textures.insert(tex.texture().index());
        }
        if let Some(tex) = mat.pbr_metallic_roughness().metallic_roughness_texture() {
            linear_textures.insert(tex.texture().index());
        }
    }

    let buffers = load_buffers_data(&model)?;
    let textures = load_textures(&model, &buffers, &linear_textures)
        .into_iter()
        .map(|tex| {
            let id = TextureId(Uuid::new_v4());
            scene.assets.textures.insert(
                id,
                // Image::from_path("gui/assets/uv_checker.png")
                //     .unwrap()
                //     .to_texture(&device, &queue),
                tex.as_texture(device, queue, &Default::default()),
            );
            id
        })
        .collect();

    for node in &json.nodes {
        if let Some(index) = node.camera {
            scene.original.camera = load_camera(json, node, index);
        }

        if let Some(index) = node.mesh {
            let (mesh, mat) = load_mesh(json, node, index, &buffers, &textures);

            let sm = StaticMesh {
                mesh: MeshInstanceId(Uuid::new_v4()),
                material: MaterialInstanceId(Uuid::new_v4()),
            };
            scene.assets.meshes.insert(sm.mesh, mesh);
            scene.original.materials.insert(sm.material, Rc::new(mat));
            scene.static_meshes.push(sm);
        }

        if let Some(light) = node
            .extensions
            .as_ref()
            .and_then(|ext| ext.khr_lights_punctual.clone())
        {
            let (dir, point, spot) = load_light(json, node, light.light);
            if let Some(dir) = dir {
                scene.original.dir_lights.insert(Uuid::new_v4(), dir);
            }
            if let Some(point) = point {
                scene.original.point_lights.insert(Uuid::new_v4(), point);
            }
            if let Some(spot) = spot {
                scene.original.spot_lights.insert(Uuid::new_v4(), spot);
            }
        }
    }

    Ok(scene)
}

const VALID_MIME_TYPES: &[&str] = &["application/octet-stream", "application/gltf-buffer"];

fn load_buffers_data(model: &Gltf) -> GltfLoadResult<Vec<Vec<u8>>> {
    let mut data = Vec::with_capacity(model.buffers().len());
    for buffer in model.buffers() {
        match buffer.source() {
            gltf::buffer::Source::Bin => {
                if let Some(blob) = model.blob.as_deref() {
                    data.push(blob.into());
                } else {
                    return Err(GltfLoadError::MissingBlob);
                }
            }
            gltf::buffer::Source::Uri(uri) => {
                let uri = percent_encoding::percent_decode_str(uri)
                    .decode_utf8()
                    .unwrap();
                let uri = uri.as_ref();
                let buffer_bytes = match DataUri::parse(uri) {
                    Ok(data_uri) if VALID_MIME_TYPES.contains(&data_uri.mime_type) => {
                        data_uri.decode()?
                    }
                    Ok(_) => return Err(GltfLoadError::BufferFormatUnsupported),
                    Err(()) => std::fs::read(uri)?,
                };
                data.push(buffer_bytes);
            }
        }
    }

    Ok(data)
}

fn load_textures(
    model: &Gltf,
    buffers: &Vec<Vec<u8>>,
    linear_textures: &HashSet<usize>,
) -> Vec<Image> {
    let mut textures = Vec::with_capacity(model.textures().len());
    for (index, texture) in model.textures().enumerate() {
        match texture.source().source() {
            gltf::image::Source::View { view, mime_type } => {
                let format = match mime_type.to_ascii_lowercase().as_str() {
                    "image/avif" => ImageFormat::Avif,
                    "image/bmp" | "image/x-bmp" => ImageFormat::Bmp,
                    "image/vnd-ms.dds" => ImageFormat::Dds,
                    "image/vnd.radiance" => ImageFormat::Hdr,
                    "image/gif" => ImageFormat::Gif,
                    "image/x-icon" => ImageFormat::Ico,
                    "image/jpeg" => ImageFormat::Jpeg,
                    "image/png" => ImageFormat::Png,
                    "image/x-exr" => ImageFormat::OpenExr,
                    "image/x-portable-bitmap"
                    | "image/x-portable-graymap"
                    | "image/x-portable-pixmap"
                    | "image/x-portable-anymap" => ImageFormat::Pnm,
                    "image/x-targa" | "image/x-tga" => ImageFormat::Tga,
                    "image/tiff" => ImageFormat::Tiff,
                    "image/webp" => ImageFormat::WebP,
                    _ => panic!(),
                };

                let image = Image::from_buffer(
                    &buffers[view.buffer().index()][view.offset()..view.offset() + view.length()],
                    format,
                    !linear_textures.contains(&index),
                );

                textures.push(image);
            }
            gltf::image::Source::Uri { uri, .. } => {
                let uri = percent_encoding::percent_decode_str(uri)
                    .decode_utf8()
                    .unwrap();
                textures.push(Image::from_path(uri.as_ref()).unwrap());
            }
        }
    }
    textures
}

fn load_camera(json: &Root, node: &Node, index: Index<gltf::json::Camera>) -> Camera {
    let camera = json.get(index).unwrap();
    Camera {
        transform: Transform {
            translation: node.translation.unwrap_or_default().into(),
            rotation: Quat::from_array(node.rotation.unwrap_or_default().0),
            scale: Vec3::ONE,
        },
        projection: if let Some(proj) = &camera.orthographic {
            CameraProjection::Orthographic(OrthographicProjection::symmetric(
                proj.xmag, proj.ymag, proj.znear, proj.zfar,
            ))
        } else if let Some(proj) = &camera.perspective {
            CameraProjection::Perspective(PerspectiveProjection {
                fov: proj.yfov,
                aspect_ratio: proj.aspect_ratio.unwrap(),
                near: proj.znear,
                far: proj.zfar.unwrap(),
            })
        } else {
            unreachable!()
        },
        exposure: Exposure::default(),
    }
}

fn load_light(
    json: &Root,
    node: &Node,
    light: Index<gltf::json::extensions::scene::khr_lights_punctual::Light>,
) -> (
    Option<GpuDirectionalLight>,
    Option<GpuPointLight>,
    Option<GpuSpotLight>,
) {
    let light = json.get(light).unwrap();

    match light.type_.unwrap() {
        gltf::json::extensions::scene::khr_lights_punctual::Type::Directional => (
            Some(GpuDirectionalLight {
                direction: node
                    .rotation
                    .map(|r| Quat::from_array(r.0))
                    .unwrap_or_default()
                    .mul_vec3(Vec3::Z),
                color: light.color.into(),
                intensity: light.intensity,
                radius: 1.,
            }),
            None,
            None,
        ),
        gltf::json::extensions::scene::khr_lights_punctual::Type::Point => (
            None,
            Some(GpuPointLight {
                position: node.translation.unwrap_or_default().into(),
                color: light.color.into(),
                intensity: light.intensity,
                radius: 1.,
            }),
            None,
        ),
        gltf::json::extensions::scene::khr_lights_punctual::Type::Spot => {
            let spot = light.spot.as_ref().unwrap();
            (
                None,
                None,
                Some(GpuSpotLight {
                    position: node.translation.unwrap_or_default().into(),
                    direction: node
                        .rotation
                        .map(|r| Quat::from_array(r.0))
                        .unwrap_or_default()
                        .mul_vec3(Vec3::Z),
                    color: light.color.into(),
                    intensity: light.intensity,
                    radius: 1.,
                    inner_angle: spot.inner_cone_angle,
                    outer_angle: spot.outer_cone_angle,
                }),
            )
        }
    }
}

fn load_mesh(
    json: &Root,
    node: &Node,
    index: Index<gltf::json::Mesh>,
    buffers: &Vec<Vec<u8>>,
    textures: &Vec<TextureId>,
) -> (Mesh, PbrMaterial) {
    let gltf_mesh = json.get(index).unwrap();

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut tangents = Vec::new();
    let mut texcoords = Vec::new();

    let material = load_material(
        json,
        gltf_mesh.primitives.iter().next().and_then(|p| p.material),
        textures,
    );

    let mut mesh = Mesh::new();

    for primitive in &gltf_mesh.primitives {
        let indices = primitive.indices.and_then(|i| json.get(i)).map(|acc| {
            let view = json.get(acc.buffer_view.unwrap()).unwrap();
            let offset = view.byte_offset.unwrap_or_default().0 as usize;
            let length = view.byte_length.0 as usize;
            let buffer = &buffers[view.buffer.value() as usize][offset..offset + length];

            match acc.component_type.unwrap().0 {
                DataType::U16 => MeshIndices::UInt16(bytemuck::cast_slice(buffer).to_owned()),
                DataType::U32 => MeshIndices::UInt32(bytemuck::cast_slice(buffer).to_owned()),
                _ => unreachable!(),
            }
        });

        if let Some(indices) = indices {
            mesh.insert_indices(indices);
        }

        for (semantic, accessor) in &primitive.attributes {
            assert_eq!(primitive.mode.unwrap(), gltf::mesh::Mode::Triangles);

            let semantic = semantic.as_ref().unwrap();
            let accessor = json.get(*accessor).unwrap();
            let data_type = accessor.component_type.unwrap().0;

            let view = json.get(accessor.buffer_view.unwrap()).unwrap();
            let offset = view.byte_offset.unwrap_or_default().0 as usize;
            let length = view.byte_length.0 as usize;
            let buffer = &buffers[view.buffer.value()][offset..offset + length];

            match semantic {
                Semantic::Positions => {
                    assert_eq!(data_type, DataType::F32);
                    positions.extend(
                        bytemuck::cast_slice(buffer)
                            .chunks_exact(3)
                            .map(Vec3::from_slice),
                    );
                }
                Semantic::Normals => {
                    assert_eq!(data_type, DataType::F32);
                    normals.extend(
                        bytemuck::cast_slice(buffer)
                            .chunks_exact(3)
                            .map(Vec3::from_slice),
                    );
                }
                Semantic::Tangents => {
                    assert_eq!(data_type, DataType::F32);
                    tangents.extend(
                        bytemuck::cast_slice(buffer)
                            .chunks_exact(4)
                            .map(Vec4::from_slice),
                    );
                }
                Semantic::Colors(_) => todo!(),
                Semantic::TexCoords(0) => {
                    assert_eq!(data_type, DataType::F32);
                    texcoords.extend(
                        bytemuck::cast_slice(buffer)
                            .chunks_exact(2)
                            .map(Vec2::from_slice),
                    );
                }
                Semantic::TexCoords(1) => todo!(),
                Semantic::TexCoords(_) => todo!(),
                Semantic::Joints(_) => todo!(),
                Semantic::Weights(_) => todo!(),
            }
        }
    }

    mesh.insert_attribute(
        Mesh::POSITION_ATTR,
        MeshVertexAttributeData::Float32x3(positions),
    )
    .insert_attribute(
        Mesh::NORMAL_ATTR,
        MeshVertexAttributeData::Float32x3(normals),
    )
    .insert_attribute(
        Mesh::TEX_COORDS_ATTR,
        MeshVertexAttributeData::Float32x2(texcoords),
    );

    if tangents.is_empty() {
        mesh.recalculate_tangent();
    } else {
        mesh.insert_attribute(
            Mesh::TANGENT_ATTR,
            MeshVertexAttributeData::Float32x4(tangents),
        );
    }

    mesh.transform(Mat4::from_scale_rotation_translation(
        node.scale.map(Vec3::from_array).unwrap_or(Vec3::ONE),
        node.rotation
            .map(|r| Quat::from_array(r.0))
            .unwrap_or_default(),
        node.translation.unwrap_or_default().into(),
    ));

    (mesh, material)
}

fn load_material(
    json: &Root,
    index: Option<Index<gltf::json::Material>>,
    textures: &Vec<TextureId>,
) -> PbrMaterial {
    let Some(index) = index else {
        return PbrMaterial::default();
    };

    let material = json.get(index).unwrap();
    let met_rough = &material.pbr_metallic_roughness;

    PbrMaterial {
        base_color: Srgb::from_components((
            met_rough.base_color_factor.0[0],
            met_rough.base_color_factor.0[1],
            met_rough.base_color_factor.0[2],
        )),
        tex_base_color: met_rough
            .base_color_texture
            .as_ref()
            .map(|info| textures[info.index.value()]),
        tex_normal: material
            .normal_texture
            .as_ref()
            .map(|info| textures[info.index.value()]),
        roughness: met_rough.roughness_factor.0,
        metallic: met_rough.metallic_factor.0,
        reflectance: 0.5,
    }
}
