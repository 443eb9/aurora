use std::{collections::HashMap, num::NonZeroU64};

use aurora_derive::ShaderData;

use bytemuck::{Pod, Zeroable};

use glam::Vec3;

use naga_oil::compose::{
    ComposableModuleDefinition, ComposableModuleDescriptor, Composer, ComposerError,
    NagaModuleDescriptor, ShaderDefValue,
};

use wgpu::{naga::Module, *};

pub trait ShaderData: Sized + Pod {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    #[inline]
    fn min_binding_size() -> Option<NonZeroU64> {
        Some(unsafe { NonZeroU64::new_unchecked(std::mem::size_of::<Self>() as u64) })
    }
}

pub struct RenderTargets<'a> {
    pub color: &'a TextureView,
    pub depth: Option<&'a TextureView>,
}

#[derive(Default)]
pub struct OwnedRenderPassDescriptor<'tex, 'desc> {
    pub label: Label<'desc>,
    pub color_attachments: Box<[Option<RenderPassColorAttachment<'tex>>]>,
    pub depth_stencil_attachment: Option<RenderPassDepthStencilAttachment<'tex>>,
    pub timestamp_writes: Option<RenderPassTimestampWrites<'desc>>,
    pub occlusion_query_set: Option<&'tex QuerySet>,
}

pub struct OwnedBindGroups<'a> {
    pub value: Vec<(&'a BindGroup, Option<Box<[u32]>>)>,
}

pub struct ComposableShader<'a> {
    main: &'a str,
    main_path: &'a str,
    composer: Composer,
}

impl<'a> ComposableShader<'a> {
    pub fn new(main: &'a str, main_path: &'a str) -> Self {
        Self {
            main,
            main_path,
            composer: Composer::default(),
        }
    }

    pub fn add_shader(
        &mut self,
        shader: &str,
        path: &str,
    ) -> Result<&ComposableModuleDefinition, ComposerError> {
        self.composer
            .add_composable_module(ComposableModuleDescriptor {
                source: shader,
                file_path: path,
                ..Default::default()
            })
    }

    pub fn compose(
        &mut self,
        shader_defs: HashMap<String, ShaderDefValue>,
    ) -> Result<Module, ComposerError> {
        self.composer.make_naga_module(NagaModuleDescriptor {
            source: &self.main,
            file_path: &self.main_path,
            shader_defs,
            ..Default::default()
        })
    }
}

pub struct GpuBinding {
    pub bind_group: Option<BindGroup>,
    pub layout: BindGroupLayout,
}

impl GpuBinding {
    pub fn new(layout: BindGroupLayout) -> Self {
        Self {
            bind_group: None,
            layout,
        }
    }

    pub fn bind<const N: usize>(&mut self, device: &Device, bindings: [BindingResource; N]) {
        self.bind_group = Some(
            device.create_bind_group(&BindGroupDescriptor {
                label: None,
                layout: &self.layout,
                entries: &bindings
                    .into_iter()
                    .enumerate()
                    .map(|(binding, resource)| BindGroupEntry {
                        binding: binding as u32,
                        resource,
                    })
                    .collect::<Vec<_>>(),
            }),
        );
    }
}

#[derive(ShaderData, Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
}
