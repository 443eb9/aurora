use std::path::Path;

use aurora_core::render::mesh::{Mesh, MeshVertexAttributeData};

pub fn mesh_from_obj(path: impl AsRef<Path>) -> Vec<Mesh> {
    let mut source = Vec::new();
    std::io::Read::read_to_end(&mut std::fs::File::open(path).unwrap(), &mut source).unwrap();
    let obj = obj::ObjData::load_buf(&source[..]).unwrap();

    let mut meshes = Vec::new();

    for object in obj.objects {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut texcoords = Vec::new();

        for group in object.groups {
            for poly in group.polys {
                for end_index in 2..poly.0.len() {
                    for &index in &[0, end_index - 1, end_index] {
                        let obj::IndexTuple(position_id, Some(texture_id), Some(normal_id)) =
                            poly.0[index]
                        else {
                            unreachable!()
                        };

                        positions.push(obj.position[position_id].into());
                        normals.push(obj.normal[normal_id].into());
                        texcoords.push(obj.texture[texture_id].into());
                    }
                }
            }
        }

        let mut mesh = Mesh::new()
            .with_attribute(
                Mesh::POSITION_ATTR,
                MeshVertexAttributeData::Float32x3(positions),
            )
            .with_attribute(
                Mesh::NORMAL_ATTR,
                MeshVertexAttributeData::Float32x3(normals),
            )
            .with_attribute(
                Mesh::TEX_COORDS_ATTR,
                MeshVertexAttributeData::Float32x2(texcoords),
            );
        mesh.recalculate_tangent();
        meshes.push(mesh);
    }

    meshes
}
