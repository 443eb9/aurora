use std::path::Path;

use bytemuck::NoUninit;
use encase::{internal::WriteInto, DynamicStorageBuffer, ShaderType};
use glam::{Mat4, UVec2, Vec3};
use image::{DynamicImage, ImageFormat, ImageResult};
use uuid::Uuid;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt, TextureDataOrder},
    BindingResource, Buffer, BufferBinding, BufferDescriptor, BufferUsages, Device, Extent3d,
    ImageCopyTexture, Origin3d, Queue, Texture, TextureAspect, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView,
};

use crate::{
    render::{
        mesh::StaticMesh,
        scene::{MaterialTypeId, TextureId},
    },
    util::cube::CUBE_MAP_OFFSETS,
    PostProcessChain,
};

pub const POST_PROCESS_COLOR_LAYOUT_UUID: MaterialTypeId =
    MaterialTypeId(Uuid::from_u128(374318654136541653489410561064));
pub const POST_PROCESS_DEPTH_LAYOUT_UUID: MaterialTypeId =
    MaterialTypeId(Uuid::from_u128(887897413248965416140604016399654));

pub const DUMMY_2D_TEX: TextureId = TextureId(Uuid::from_u128(8674167498640649160513219685401));

pub struct RenderTargets<'a> {
    pub color_format: TextureFormat,
    pub color: TextureView,
    pub surface: TextureView,
    pub depth_format: Option<TextureFormat>,
    pub depth: Option<TextureView>,
    pub post_process_chain: &'a PostProcessChain,
    pub size: UVec2,
}

pub struct DynamicGpuBuffer {
    raw: DynamicStorageBuffer<Vec<u8>>,
    buffer: Option<Buffer>,
    changed: bool,
    usage: BufferUsages,
}

impl DynamicGpuBuffer {
    pub fn new(usage: BufferUsages) -> Self {
        Self {
            raw: DynamicStorageBuffer::new(Vec::new()),
            buffer: None,
            changed: true,
            usage: usage | BufferUsages::COPY_DST,
        }
    }

    pub fn new_with_alignment(usage: BufferUsages, alignment: u64) -> Self {
        Self {
            raw: DynamicStorageBuffer::new_with_alignment(Vec::new(), alignment),
            buffer: None,
            changed: true,
            usage: usage | BufferUsages::COPY_DST,
        }
    }

    pub fn set(&mut self, data: Vec<u8>) {
        self.raw = DynamicStorageBuffer::new(data);
        self.changed = true;
    }

    pub fn push<E: ShaderType + WriteInto>(&mut self, data: &E) -> u32 {
        self.raw.write(data).unwrap() as u32
    }

    pub fn usage(&self) -> &BufferUsages {
        &self.usage
    }

    pub fn usage_mut(&mut self) -> &mut BufferUsages {
        self.changed = true;
        &mut self.usage
    }

    pub fn write<E: ShaderType + WriteInto>(&mut self, device: &Device, queue: &Queue) {
        let capacity = self.buffer.as_ref().map(|b| b.size()).unwrap_or(0);
        let size = self.raw.as_ref().len() as u64;

        if capacity < size || self.changed {
            if size == 0 {
                self.buffer = Some(device.create_buffer(&BufferDescriptor {
                    label: None,
                    size: E::min_size().get(),
                    usage: self.usage,
                    mapped_at_creation: false,
                }));
            } else {
                self.buffer = Some(device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    usage: self.usage,
                    contents: self.raw.as_ref(),
                }));
            }
            self.changed = false;
        } else if let Some(buffer) = &self.buffer {
            queue.write_buffer(buffer, 0, self.raw.as_ref());
        }
    }

    pub fn clear(&mut self) {
        self.raw.as_mut().clear();
        self.raw.set_offset(0);
    }

    pub fn binding<E: ShaderType>(&self) -> Option<BindingResource> {
        Some(BindingResource::Buffer(BufferBinding {
            buffer: self.buffer()?,
            offset: 0,
            size: Some(E::min_size()),
        }))
    }

    pub fn entire_binding(&self) -> Option<BindingResource> {
        self.buffer.as_ref().map(|b| b.as_entire_binding())
    }

    pub fn buffer(&self) -> Option<&Buffer> {
        self.buffer.as_ref()
    }

    pub fn len_bytes(&self) -> usize {
        self.raw.as_ref().len()
    }
}

#[derive(Default)]
pub struct ImageTextureDescriptor<'a> {
    pub label: Option<&'a str>,
    pub mip_level_count: Option<u32>,
    pub sample_count: Option<u32>,
    pub dimension: Option<TextureDimension>,
    pub usage: Option<TextureUsages>,
    pub view_formats: Option<&'a [TextureFormat]>,
    pub data_order: Option<TextureDataOrder>,
}

pub struct Image {
    width: u32,
    height: u32,
    format: TextureFormat,
    buffer: Vec<u8>,
}

impl Image {
    pub fn from_dynamic(dyn_image: DynamicImage, is_srgb: bool) -> Self {
        let width;
        let height;
        let fmt;
        let data;

        match dyn_image {
            DynamicImage::ImageLuma8(img) => {
                let img = DynamicImage::ImageLuma8(img).into_rgba8();
                width = img.width();
                height = img.height();
                fmt = if is_srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                };

                data = img.into_raw();
            }
            DynamicImage::ImageLumaA8(img) => {
                let img = DynamicImage::ImageLumaA8(img).into_rgba8();
                width = img.width();
                height = img.height();
                fmt = if is_srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                };

                data = img.into_raw();
            }
            DynamicImage::ImageRgb8(img) => {
                let img = DynamicImage::ImageRgb8(img).into_rgba8();
                width = img.width();
                height = img.height();
                fmt = if is_srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                };

                data = img.into_raw();
            }
            DynamicImage::ImageRgba8(img) => {
                let img = DynamicImage::ImageRgba8(img).into_rgba8();
                width = img.width();
                height = img.height();
                fmt = if is_srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                };

                data = img.into_raw();
            }
            DynamicImage::ImageLuma16(img) => {
                width = img.width();
                height = img.height();
                fmt = TextureFormat::R16Uint;
                data = bytemuck::cast_slice(img.as_raw()).to_owned();
            }
            DynamicImage::ImageLumaA16(img) => {
                width = img.width();
                height = img.height();
                fmt = TextureFormat::R16Uint;
                data = bytemuck::cast_slice(img.as_raw()).to_owned();
            }
            DynamicImage::ImageRgb16(img) => {
                width = img.width();
                height = img.height();
                fmt = TextureFormat::R16Unorm;
                data = bytemuck::cast_slice(img.as_raw()).to_owned();
            }
            DynamicImage::ImageRgba16(img) => {
                width = img.width();
                height = img.height();
                fmt = TextureFormat::R16Unorm;
                data = bytemuck::cast_slice(img.as_raw()).to_owned();
            }
            DynamicImage::ImageRgb32F(img) => {
                width = img.width();
                height = img.height();
                fmt = TextureFormat::Rgba32Float;
                let mut local_data = Vec::with_capacity(
                    width as usize * height as usize * fmt.block_copy_size(None).unwrap() as usize,
                );

                for pixel in img.into_raw().chunks_exact(3) {
                    local_data.extend_from_slice(&pixel[0].to_ne_bytes());
                    local_data.extend_from_slice(&pixel[1].to_ne_bytes());
                    local_data.extend_from_slice(&pixel[2].to_ne_bytes());
                    local_data.extend_from_slice(&1f32.to_ne_bytes());
                }
                data = local_data;
            }
            DynamicImage::ImageRgba32F(img) => {
                width = img.width();
                height = img.height();
                fmt = TextureFormat::Rgba32Float;
                data = bytemuck::cast_slice(img.as_raw()).to_owned();
            }
            _ => unreachable!(),
        }

        Self {
            width,
            height,
            format: fmt,
            buffer: data,
        }
    }

    pub fn from_path(path: impl AsRef<Path>) -> ImageResult<Self> {
        let img = image::open(path)?;
        Ok(Self::from_dynamic(img, false))
    }

    pub fn from_raw_parts(buffer: Vec<u8>, format: TextureFormat, width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format,
            buffer,
        }
    }

    pub fn from_buffer(data: &[u8], format: ImageFormat, is_srgb: bool) -> Self {
        let mut reader = image::ImageReader::new(std::io::Cursor::new(data));
        reader.set_format(format);
        reader.no_limits();
        let dyn_image = reader.decode().unwrap();
        Self::from_dynamic(dyn_image, is_srgb)
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn to_texture(
        &self,
        device: &Device,
        queue: &Queue,
        desc: &ImageTextureDescriptor,
    ) -> Texture {
        device.create_texture_with_data(
            queue,
            &TextureDescriptor {
                label: desc.label,
                size: Extent3d {
                    width: self.width,
                    height: self.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: desc.mip_level_count.unwrap_or(1),
                sample_count: desc.sample_count.unwrap_or(1),
                dimension: desc.dimension.unwrap_or(TextureDimension::D2),
                format: self.format,
                usage: desc.usage.unwrap_or_else(TextureUsages::empty)
                    | TextureUsages::TEXTURE_BINDING,
                view_formats: desc.view_formats.unwrap_or_default(),
            },
            desc.data_order.unwrap_or_default(),
            &self.buffer,
        )
    }

    pub fn to_cube_map(
        &self,
        device: &Device,
        queue: &Queue,
        desc: &ImageTextureDescriptor,
    ) -> Texture {
        assert_eq!(self.width / 4, self.height / 3, "Invalid cubemap.");
        let face_size = self.width / 4;
        let mut cmd = device.create_command_encoder(&Default::default());
        let cube_map = device.create_texture(&TextureDescriptor {
            label: desc.label,
            size: Extent3d {
                width: face_size,
                height: face_size,
                depth_or_array_layers: 6,
            },
            mip_level_count: desc.mip_level_count.unwrap_or(1),
            sample_count: desc.sample_count.unwrap_or(1),
            dimension: desc.dimension.unwrap_or(TextureDimension::D2),
            format: self.format,
            usage: desc.usage.unwrap_or_else(TextureUsages::empty)
                | TextureUsages::COPY_DST
                | TextureUsages::TEXTURE_BINDING,
            view_formats: desc.view_formats.unwrap_or_default(),
        });

        let flat = self.to_texture(
            device,
            queue,
            &ImageTextureDescriptor {
                dimension: None,
                usage: Some(
                    desc.usage.unwrap_or_else(TextureUsages::empty) | TextureUsages::COPY_SRC,
                ),
                ..*desc
            },
        );

        for (index, offset) in CUBE_MAP_OFFSETS.into_iter().enumerate() {
            cmd.copy_texture_to_texture(
                ImageCopyTexture {
                    texture: &flat,
                    aspect: TextureAspect::All,
                    mip_level: 0,
                    origin: Origin3d {
                        x: offset.x * face_size,
                        y: offset.y * face_size,
                        z: 0,
                    },
                },
                ImageCopyTexture {
                    texture: &cube_map,
                    mip_level: 0,
                    origin: Origin3d {
                        x: 0,
                        y: 0,
                        z: index as u32,
                    },
                    aspect: TextureAspect::All,
                },
                Extent3d {
                    width: face_size,
                    height: face_size,
                    depth_or_array_layers: 1,
                },
            );
        }
        queue.submit([cmd.finish()]);
        cube_map
    }
}

#[derive(Clone)]
pub struct RenderMesh {
    pub mesh: StaticMesh,
    pub offset: Option<u32>,
}

#[derive(ShaderType, NoUninit, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct GpuCamera {
    pub view: Mat4,
    pub inv_view: Mat4,
    pub proj: Mat4,
    pub inv_proj: Mat4,
    pub position_ws: Vec3,
    pub exposure: f32,
}

#[derive(ShaderType)]
pub struct GpuSceneDesc {
    pub dir_lights: u32,
    pub point_lights: u32,
    pub spot_lights: u32,
}

#[derive(ShaderType)]
pub struct GpuDirectionalLight {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub radius: f32,
}

#[derive(ShaderType)]
pub struct GpuPointLight {
    pub position: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub radius: f32,
}

#[derive(ShaderType)]
pub struct GpuSpotLight {
    pub position: Vec3,
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub radius: f32,
    pub inner_angle: f32,
    pub outer_angle: f32,
}
