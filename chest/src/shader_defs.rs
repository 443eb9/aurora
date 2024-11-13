use aurora_derive::ShaderDefEnum;

#[derive(ShaderDefEnum, Default)]
pub enum PbrSpecular {
    Beckmann,
    BlinnPhong,
    #[default]
    #[def_name = "GGX"]
    GGX,
    #[def_name = "GTR"]
    GTR,
}

#[derive(ShaderDefEnum, Default)]
pub enum PbrDiffuse {
    Lambert,
    #[default]
    Burley,
}

#[derive(ShaderDefEnum, Default)]
pub enum ShadowFiltering {
    #[def_name = "PCF"]
    PCF,
    #[default]
    #[def_name = "PCSS"]
    PCSS,
}
