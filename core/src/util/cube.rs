use glam::{UVec2, Vec3};
use uuid::Uuid;

// From Bevy
pub struct CubeMapFace {
    pub target: Vec3,
    pub up: Vec3,
    pub id: Uuid,
}

// see https://www.khronos.org/opengl/wiki/Cubemap_Texture
pub const CUBE_MAP_FACES: [CubeMapFace; 6] = [
    // 0 	GL_TEXTURE_CUBE_MAP_POSITIVE_X
    CubeMapFace {
        target: Vec3::NEG_X,
        up: Vec3::NEG_Y,
        id: Uuid::from_u128(987456123548145124610214551202),
    },
    // 1 	GL_TEXTURE_CUBE_MAP_NEGATIVE_X
    CubeMapFace {
        target: Vec3::X,
        up: Vec3::NEG_Y,
        id: Uuid::from_u128(654653154451204512300215485120),
    },
    // 2 	GL_TEXTURE_CUBE_MAP_POSITIVE_Y
    CubeMapFace {
        target: Vec3::NEG_Y,
        up: Vec3::Z,
        id: Uuid::from_u128(120014512300230205685230),
    },
    // 3 	GL_TEXTURE_CUBE_MAP_NEGATIVE_Y
    CubeMapFace {
        target: Vec3::Y,
        up: Vec3::NEG_Z,
        id: Uuid::from_u128(431105314304087942300),
    },
    // 4 	GL_TEXTURE_CUBE_MAP_POSITIVE_Z
    CubeMapFace {
        target: Vec3::NEG_Z,
        up: Vec3::NEG_Y,
        id: Uuid::from_u128(065132643512148745120548),
    },
    // 5 	GL_TEXTURE_CUBE_MAP_NEGATIVE_Z
    CubeMapFace {
        target: Vec3::Z,
        up: Vec3::NEG_Y,
        id: Uuid::from_u128(1485120178465129865312),
    },
];

/// Offsets of cube map faces on a 2d texture.
/// 
/// ** +Y ** **
/// -X +Z +X -Z
/// ** -Y ** **
pub const CUBE_MAP_OFFSETS: [UVec2; 6] = [
    UVec2 { x: 2, y: 1 },
    UVec2 { x: 0, y: 1 },
    UVec2 { x: 1, y: 0 },
    UVec2 { x: 1, y: 2 },
    UVec2 { x: 1, y: 1 },
    UVec2 { x: 3, y: 1 },
];
