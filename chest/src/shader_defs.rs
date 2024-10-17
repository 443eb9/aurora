use aurora_derive::ShaderDefEnum;

#[derive(ShaderDefEnum)]
pub enum PbrSpecular {
    Beckmann,
    BlinnPhong,
    #[def_name = "GGX"]
    GGX,
    #[def_name = "GTR"]
    GTR,
}

#[derive(ShaderDefEnum)]
pub enum PbrDiffuse {
    Lambert,
    Burley,
}

#[derive(ShaderDefEnum)]
pub enum ShadowFilter {
    Pcf,
}
