use std::any::TypeId;

use glam::Vec3;
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
