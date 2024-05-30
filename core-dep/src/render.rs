use std::{collections::HashMap, num::NonZeroU64};

use bytemuck::Pod;

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

#[derive(Default, Clone)]
pub struct OwnedRenderPassDescriptor<'desc> {
    pub label: Label<'desc>,
    pub color_attachments: Box<[Option<RenderPassColorAttachment<'desc>>]>,
    pub depth_stencil_attachment: Option<RenderPassDepthStencilAttachment<'desc>>,
    pub timestamp_writes: Option<RenderPassTimestampWrites<'desc>>,
    pub occlusion_query_set: Option<&'desc QuerySet>,
}

pub struct ComposableShader {
    composer: Composer,
}

impl ComposableShader {
    pub fn new() -> Self {
        Self {
            composer: Composer::default(),
        }
    }

    pub fn add_shader(
        &mut self,
        shader: &str,
    ) -> Result<&ComposableModuleDefinition, ComposerError> {
        self.composer
            .add_composable_module(ComposableModuleDescriptor {
                source: shader,
                ..Default::default()
            })
    }

    pub fn compose(
        &mut self,
        main: &str,
        shader_defs: HashMap<String, ShaderDefValue>,
    ) -> Result<Module, ComposerError> {
        self.composer.make_naga_module(NagaModuleDescriptor {
            source: &main,
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
