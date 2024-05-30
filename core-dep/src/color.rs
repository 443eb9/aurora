use glam::Vec4;

#[derive(Debug, Clone, Copy)]
pub struct SrgbaColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Default for SrgbaColor {
    fn default() -> Self {
        Self {
            r: 1.,
            g: 1.,
            b: 1.,
            a: 1.,
        }
    }
}

impl SrgbaColor {
    pub const TRANSPARENT: Self = Self {
        r: 0.,
        g: 0.,
        b: 0.,
        a: 0.,
    };

    pub const BLACK: Self = Self {
        r: 0.,
        g: 0.,
        b: 0.,
        a: 1.,
    };

    pub const WHITE: Self = Self {
        r: 1.,
        g: 1.,
        b: 1.,
        a: 1.,
    };

    pub fn to_linear_rgba(self) -> LinearRgbaColor {
        LinearRgbaColor {
            r: Self::gamma(self.r),
            g: Self::gamma(self.g),
            b: Self::gamma(self.b),
            a: Self::gamma(self.a),
        }
    }

    // From bevy_color
    fn gamma(x: f32) -> f32 {
        if x <= 0.04045 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    }

    // From bevy_color
    fn gamma_inv(x: f32) -> f32 {
        if x <= 0.0031308 {
            x * 12.92
        } else {
            (1.055 * x.powf(1. / 2.4)) - 0.055
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LinearRgbaColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Default for LinearRgbaColor {
    fn default() -> Self {
        Self {
            r: 1.,
            g: 1.,
            b: 1.,
            a: 1.,
        }
    }
}

impl LinearRgbaColor {
    pub const TRANSPARENT: Self = Self {
        r: 0.,
        g: 0.,
        b: 0.,
        a: 0.,
    };

    pub const BLACK: Self = Self {
        r: 0.,
        g: 0.,
        b: 0.,
        a: 1.,
    };

    pub const WHITE: Self = Self {
        r: 1.,
        g: 1.,
        b: 1.,
        a: 1.,
    };
    
    pub fn to_srgba(self) -> SrgbaColor {
        SrgbaColor {
            r: SrgbaColor::gamma_inv(self.r),
            g: SrgbaColor::gamma_inv(self.g),
            b: SrgbaColor::gamma_inv(self.b),
            a: SrgbaColor::gamma_inv(self.a),
        }
    }
}

impl Into<Vec4> for SrgbaColor {
    fn into(self) -> Vec4 {
        unsafe { std::mem::transmute(self) }
    }
}

impl Into<Vec4> for LinearRgbaColor {
    fn into(self) -> Vec4 {
        unsafe { std::mem::transmute(self) }
    }
}

impl Into<wgpu::Color> for SrgbaColor {
    fn into(self) -> wgpu::Color {
        wgpu::Color {
            r: self.r as f64,
            g: self.g as f64,
            b: self.b as f64,
            a: self.a as f64,
        }
    }
}

impl Into<wgpu::Color> for LinearRgbaColor {
    fn into(self) -> wgpu::Color {
        wgpu::Color {
            r: self.r as f64,
            g: self.g as f64,
            b: self.b as f64,
            a: self.a as f64,
        }
    }
}
