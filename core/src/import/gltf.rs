use std::path::Path;

use gltf::{accessor::DataType, buffer::Source, Accessor, Gltf, Semantic};
use thiserror::Error;

use crate::render::scene::GpuScene;

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

pub fn load_gltf(path: impl AsRef<Path>) -> GltfLoadResult<GpuScene> {
    let model = Gltf::open(path)?;
    let cameras = model.cameras().collect::<Vec<_>>();
    if cameras.len() != 1 {
        return Err(GltfLoadError::MultipleCameras(cameras.len() as u32));
    }

    let buffers = load_buffers_data(&model)?;
    let main_camera = cameras[0].clone();
    for mesh in model.meshes() {
        // let vertices = Vec::new();
        for primitive in mesh.primitives() {
            for (semantic, accessor) in primitive.attributes() {
                match semantic {
                    Semantic::Positions => {
                        assert_eq!(accessor.data_type(), DataType::F32);
                        if let Some(view) = accessor.view() {
                            let data =
                                &buffers[view.buffer().index()][view.offset()..view.length()];
                            let floats = bytemuck::cast_slice::<_, f32>(data);
                        }
                    }
                    Semantic::Normals => {
                        assert_eq!(accessor.data_type(), DataType::F32);
                    }
                    Semantic::Tangents => {
                        assert_eq!(accessor.data_type(), DataType::F32);
                    }
                    Semantic::Colors(_) => todo!(),
                    Semantic::TexCoords(_) => todo!(),
                    Semantic::Joints(_) => todo!(),
                    Semantic::Weights(_) => todo!(),
                }
            }
        }
    }
    todo!()
}

fn load_buffers_data(model: &Gltf) -> GltfLoadResult<Vec<Vec<u8>>> {
    const VALID_MIME_TYPES: &[&str] = &["application/octet-stream", "application/gltf-buffer"];

    let mut data = Vec::with_capacity(model.buffers().len());
    for buffer in model.buffers() {
        match buffer.source() {
            Source::Bin => {
                if let Some(blob) = model.blob.as_deref() {
                    data.push(blob.into());
                } else {
                    return Err(GltfLoadError::MissingBlob);
                }
            }
            Source::Uri(uri) => {
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

fn access_buffer_data(accessor: &Accessor, buffers: &Vec<Vec<u8>>) {}
