use std::{fs::File, io::Write, path::Path};

use glam::{UVec2, UVec3};

use png::ColorType;

use wgpu::*;

#[derive(Debug)]
pub enum AuroraError {
    ResourceNotReady,
}

pub fn create_texture(
    device: &Device,
    dim: UVec3,
    format: TextureFormat,
    usage: TextureUsages,
) -> (Texture, TextureView) {
    let target = device.create_texture(&TextureDescriptor {
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
    });
    let target_view = target.create_view(&TextureViewDescriptor::default());
    (target, target_view)
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

    let mut command_encoder =
        device.create_command_encoder(&CommandEncoderDescriptor { label: None });
    command_encoder.copy_texture_to_buffer(
        ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
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

    save_raw_bytes_to_image(UVec2::new(extent.width, extent.height), &texture_data, path);
}

fn save_raw_bytes_to_image(dim: UVec2, bytes: &[u8], path: impl AsRef<Path>) {
    let mut png_image = Vec::with_capacity(bytes.len());
    let mut encoder = png::Encoder::new(std::io::Cursor::new(&mut png_image), dim.x, dim.y);
    encoder.set_color(ColorType::Rgba);

    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(bytes).unwrap();
    writer.finish().unwrap();

    File::create(path).unwrap().write_all(&png_image).unwrap();
}
