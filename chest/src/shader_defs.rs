use naga_oil::compose::ShaderDefValue;

pub trait StrAsShaderDef {
    fn as_shader_def(&self) -> (String, ShaderDefValue);
}

impl StrAsShaderDef for str {
    fn as_shader_def(&self) -> (String, ShaderDefValue) {
        (self.to_string(), ShaderDefValue::Bool(true))
    }
}
