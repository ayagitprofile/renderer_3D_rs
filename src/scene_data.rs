use std::marker::PhantomData;

use smol_str::SmolStr;

use crate::{
    graphics::{material_properties::MaterialProperties, mesh::Mesh, shader::Shader, texture::Texture2D},
    transform::Transform,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceHandle<T> {
    pub index: usize,
    _marker: PhantomData<fn() -> T>,
}

pub type MeshNodeID = ResourceHandle<MeshNode>;
pub type MeshID = ResourceHandle<Mesh>;
pub type ShaderID = ResourceHandle<NamedShader>;
pub type TextureID = ResourceHandle<Texture2D>;
pub type MaterialID = ResourceHandle<Material>;

pub struct Material {
    pub material_properties: MaterialProperties,
    pub shader_ref: ShaderID,
    pub name: SmolStr,
}

impl Material {
    pub fn new(shader_ref: ShaderID, mat_props: &MaterialProperties, name: &str) -> Self {
        Self {
            material_properties: *mat_props,
            shader_ref: shader_ref,
            name: SmolStr::from(name),
        }
    }
}

pub struct NamedShader {
    pub name: String,
    pub shader: Shader,
}

pub type MeshChildrenStorage = smallvec::SmallVec<[MeshNodeID; 3]>;

pub struct MeshNode {
    pub transform: Transform,
    children_refs: MeshChildrenStorage,
    pub mesh_ref: MeshID,
    pub material_ref: MaterialID,
}

impl MeshNode {
    pub fn children(&self) -> &[MeshNodeID] {
        self.children_refs.as_slice()
    }

    pub fn new(
        transform: Transform,
        children_refs: smallvec::SmallVec<[MeshNodeID; 3]>,
        mesh_ref: MeshID,
        material_ref: MaterialID,
    ) -> Self {
        Self {
            transform,
            children_refs,
            mesh_ref,
            material_ref,
        }
    }
}

impl<T> ResourceHandle<T> {
    pub const fn new(index: usize) -> Self {
        Self {
            index,
            _marker: PhantomData,
        }
    }
}
