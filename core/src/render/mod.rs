use std::num::NonZeroU64;

use crate::WgpuRenderer;

pub mod flow;
pub mod resource;
pub mod scene;

pub trait ShaderData: Sized {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                core::mem::size_of::<Self>(),
            )
        }
    }

    #[inline]
    fn min_binding_size() -> Option<NonZeroU64> {
        Some(unsafe { NonZeroU64::new_unchecked(std::mem::size_of::<Self>() as u64) })
    }
}

pub trait Transferable {
    type GpuRepr;

    fn transfer(&self, renderer: &WgpuRenderer) -> Self::GpuRepr;
}
