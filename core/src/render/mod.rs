use naga_oil::compose::ShaderDefValue;

pub mod flow;
pub mod helper;
pub mod mesh;
pub mod resource;
pub mod scene;

pub trait ShaderDefEnum {
    fn to_def(&self) -> (String, ShaderDefValue);
}
