use std::{io::Cursor, path::Path};

use aurora_core::WgpuRenderer;
use ddsfile::{Dds, DxgiFormat};
use wgpu::{
    util::{DeviceExt, TextureDataOrder},
    Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

pub fn load_dds_texture(renderer: &WgpuRenderer, path: impl AsRef<Path>) -> Texture {
    let dds = Dds::read(&mut Cursor::new(std::fs::read(path).unwrap())).unwrap();
    assert_eq!(
        dds.get_dxgi_format().unwrap(),
        DxgiFormat::R9G9B9E5_SharedExp
    );

    let dds_data = dds.get_data(0).unwrap();
    assert_eq!(dds_data.as_ptr() as usize % 4, 0);

    renderer.device.create_texture_with_data(
        &renderer.queue,
        &TextureDescriptor {
            label: None,
            size: Extent3d {
                width: dds.get_width(),
                height: dds.get_height(),
                depth_or_array_layers: dds.get_depth(),
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format: TextureFormat::Rgb9e5Ufloat,
            usage: TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        TextureDataOrder::MipMajor,
        dds_data,
    )
}
