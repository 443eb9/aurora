use std::any::TypeId;

use glam::Vec3;
use naga_oil::compose::ShaderDefValue;
use palette::{rgb::Rgb, LinSrgb};
use uuid::Uuid;

pub trait TypeIdAsUuid {
    fn to_uuid(self) -> Uuid;
}

impl TypeIdAsUuid for TypeId {
    fn to_uuid(self) -> Uuid {
        unsafe { std::mem::transmute(self) }
    }
}

pub trait StrAsShaderDef {
    fn to_def(self) -> (String, ShaderDefValue);
}

impl StrAsShaderDef for &str {
    fn to_def(self) -> (String, ShaderDefValue) {
        (self.to_string(), ShaderDefValue::Bool(true))
    }
}

pub trait RgbToVec3 {
    fn to_vec3(self) -> Vec3;
}

macro_rules! impl_rgb_to_vec3 {
    ($ty: ident) => {
        impl RgbToVec3 for $ty {
            fn to_vec3(self) -> Vec3 {
                Vec3 {
                    x: self.red,
                    y: self.green,
                    z: self.blue,
                }
            }
        }
    };
}

impl_rgb_to_vec3!(Rgb);
impl_rgb_to_vec3!(LinSrgb);
