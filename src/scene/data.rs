use smol_str::SmolStr;

use crate::{
    graphics::{material_properties::MaterialProperties, mesh::Mesh, shader::Shader, texture::Texture2D, vertex},
    scene::resource_id::ResourceID,
    transform::Transform,
};

pub type MaterialID = ResourceID<Material>;
pub type TextureID = ResourceID<NamedTexture>;
pub type ShaderID = ResourceID<NamedShader>;
pub type NodeID = ResourceID<Node>;
pub type MeshID = ResourceID<Mesh>;

pub(super) type NodeIDStorage = smallvec::SmallVec<[NodeID; 3]>;
pub(super) type TextureIDStorage = smallvec::SmallVec<[TextureID; 3]>;

pub const VERTEX_LAYOUT_ATTRIBS: [vertex::Attrib; 3] =
    [vertex::Attrib::POSITION, vertex::Attrib::NORMAL, vertex::Attrib::UV];

pub struct NamedShader {
    pub shader: Shader,
    pub name: SmolStr,
}

pub struct NamedTexture {
    pub texture: Texture2D,
    pub name: SmolStr,
}

pub struct Material {
    pub(super) texture_ids: TextureIDStorage,
    pub name: SmolStr,
    pub material_properties: MaterialProperties,
    pub shader_id: ShaderID,
}

pub struct Node {
    pub transform: Transform,
    children_ids: NodeIDStorage,
    pub mesh_id: MeshID,
    pub material_id: MaterialID,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex {
    pub const fn new(position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Self {
        Self {
            position: position,
            normal: normal,
            uv: uv,
        }
    }
}

impl Node {
    pub fn children_iter(&self) -> std::slice::Iter<'_, ResourceID<Node>> {
        self.children_ids.iter()
    }

    pub fn new(
        transform: Transform,
        children_ids: smallvec::SmallVec<[NodeID; 3]>,
        mesh_id: MeshID,
        material_id: MaterialID,
    ) -> Self {
        Self {
            transform,
            children_ids,
            mesh_id,
            material_id,
        }
    }
}

impl Material {
    pub fn new(shader_id: ShaderID, mat_props: &MaterialProperties, name: &str, texture_ids: TextureIDStorage) -> Self {
        Self {
            material_properties: *mat_props,
            shader_id: shader_id,
            name: SmolStr::from(name),
            texture_ids: texture_ids,
        }
    }

    pub fn texture_iter(&self) -> std::slice::Iter<'_, ResourceID<NamedTexture>> {
        self.texture_ids.iter()
    }
}
