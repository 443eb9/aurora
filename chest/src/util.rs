use std::collections::HashMap;

use naga_oil::compose::{ComposableModuleDescriptor, Composer, ShaderDefValue};

pub fn add_shader_module(
    composer: &mut Composer,
    shader: &str,
    shader_defs: Option<HashMap<String, ShaderDefValue>>,
) {
    composer
        .add_composable_module(ComposableModuleDescriptor {
            source: shader,
            shader_defs: shader_defs.unwrap_or_default(),
            ..Default::default()
        })
        .unwrap();
}
