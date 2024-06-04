use aurora_derive::ShaderDefEnum;
use naga_oil::compose::ShaderDefValue;

pub trait StrAsShaderDef {
    fn as_shader_def(&self) -> (String, ShaderDefValue);
}

impl StrAsShaderDef for str {
    fn as_shader_def(&self) -> (String, ShaderDefValue) {
        (self.to_string(), ShaderDefValue::Bool(true))
    }
}

#[derive(ShaderDefEnum)]
pub enum PbrMaterialVariant {
    TexBaseColor,
}

#[derive(ShaderDefEnum)]
pub enum PbrNdf {
    Beckmann,
    BlinnPhong,
    #[def_name = "GGX"]
    GGX,
    #[def_name = "GTR"]
    GTR,
}

#[derive(ShaderDefEnum)]
pub enum PbrBrdf {
    Diffuse,
    Specular,
    MultiBounce,
}
