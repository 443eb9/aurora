use std::path::Path;

use glam::UVec3;
use image::RgbaImage;
use wgpu::{
    BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Device, Extent3d, Features,
    ImageCopyBuffer, ImageDataLayout, Maintain, MapMode, Queue, Texture, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
};

pub mod cube;
pub mod ext;

pub fn create_texture(
    device: &Device,
    dim: UVec3,
    format: TextureFormat,
    usage: TextureUsages,
) -> Texture {
    device.create_texture(&TextureDescriptor {
        label: None,
        size: Extent3d {
            width: dim.x,
            height: dim.y,
            depth_or_array_layers: dim.z,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: {
            if dim.z == 1 && dim.y == 1 {
                TextureDimension::D1
            } else if dim.z == 1 {
                TextureDimension::D2
            } else {
                TextureDimension::D3
            }
        },
        format,
        usage,
        view_formats: &[format],
    })
}

pub async fn save_color_texture_as_image(
    path: impl AsRef<Path>,
    texture: &Texture,
    device: &Device,
    queue: &Queue,
) {
    let extent = texture.size();
    let mut texture_data = Vec::<u8>::with_capacity((extent.width * extent.height * 4) as usize);

    let out_staging_buffer = device.create_buffer(&BufferDescriptor {
        label: None,
        size: texture_data.capacity() as u64,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut command_encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
    command_encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        ImageCopyBuffer {
            buffer: &out_staging_buffer,
            layout: ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(extent.width * 4 as u32),
                rows_per_image: Some(extent.height as u32),
            },
        },
        extent,
    );
    queue.submit(Some(command_encoder.finish()));

    let buffer_slice = out_staging_buffer.slice(..);
    let (sender, receiver) = flume::bounded(1);

    buffer_slice.map_async(MapMode::Read, move |r| sender.send(r).unwrap());
    device.poll(Maintain::wait()).panic_on_timeout();
    receiver.recv_async().await.unwrap().unwrap();

    {
        let view = buffer_slice.get_mapped_range();
        texture_data.extend_from_slice(&view[..]);
    }

    out_staging_buffer.unmap();

    RgbaImage::from_raw(extent.width, extent.height, texture_data)
        .unwrap()
        .save(path)
        .unwrap();
}

pub fn struct_to_bytes<T>(s: &T) -> &[u8] {
    unsafe { core::slice::from_raw_parts(s as *const T as *const u8, core::mem::size_of::<T>()) }
}

pub fn texel_format_to_storage(format: TextureFormat) -> &'static str {
    match format {
        TextureFormat::R32Uint => "r32uint",
        TextureFormat::R32Sint => "r32sint",
        TextureFormat::R32Float => "r32float",
        TextureFormat::Rgba8Unorm => "rgba8unorm",
        TextureFormat::Rgba8Snorm => "rgba8snorm",
        TextureFormat::Rgba8Uint => "rgba8uint",
        TextureFormat::Rgba8Sint => "rgba8sint",
        TextureFormat::Bgra8Unorm => "bgra8unorm",
        TextureFormat::Rg32Uint => "rg32uint",
        TextureFormat::Rg32Sint => "rg32sint",
        TextureFormat::Rg32Float => "rg32float",
        TextureFormat::Rgba16Uint => "rgba16uint",
        TextureFormat::Rgba16Sint => "rgba16sint",
        TextureFormat::Rgba16Float => "rgba16float",
        TextureFormat::Rgba32Uint => "rgba32uint",
        TextureFormat::Rgba32Sint => "rgba32sint",
        TextureFormat::Rgba32Float => "rgba32float",
        _ => panic!("Format not supported."),
    }
}

pub fn texel_format_to_sampled(
    format: TextureFormat,
    aspect: Option<TextureAspect>,
    device_features: Option<Features>,
) -> &'static str {
    match format.sample_type(aspect, device_features).unwrap() {
        TextureSampleType::Float { .. } => "f32",
        TextureSampleType::Depth => "depth",
        TextureSampleType::Sint => "i32",
        TextureSampleType::Uint => "u32",
    }
}
