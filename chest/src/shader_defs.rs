use aurora_derive::ShaderDefEnum;

#[derive(ShaderDefEnum)]
pub enum PbrSpecular {
    Beckmann,
    BlinnPhong,
    #[def_name = "GGX"]
    GGX,
    #[def_name = "GTR"]
    GTR,
    Anisotropic,
}

#[derive(ShaderDefEnum)]
pub enum PbrDiffuse {
    Lambert,
    Burley,
}
