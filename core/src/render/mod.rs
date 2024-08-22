use naga_oil::compose::ShaderDefValue;

use crate::WgpuRenderer;

pub mod flow;
pub mod resource;
pub mod scene;
pub mod shadow;

pub trait Transferable {
    type GpuRepr;

    fn transfer(&self, renderer: &WgpuRenderer) -> Self::GpuRepr;
}

pub trait ShaderDefEnum {
    fn to_def(self) -> (String, ShaderDefValue);
}
