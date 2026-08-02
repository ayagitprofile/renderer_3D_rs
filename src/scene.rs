use glam::Mat4;

use crate::{
    graphics::{
        self,
        buffer::Usage,
        material_properties::MaterialProperties,
        mesh::{self, Mesh},
        shader::Shader,
        texture::Texture2D,
        vertex,
    },
    timer,
    transform::Transform,
};
use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
    vec::Vec,
};

struct NamedShader {
    pub name: String,
    pub shader: Shader,
}

pub struct Scene {
    meshes: Vec<Mesh>,
    shaders: Vec<NamedShader>,
    textures: Vec<Texture2D>,
    materials: Vec<Material>,
    mesh_nodes: Vec<MeshNode>,
    mesh_node_roots: Vec<MeshNodeID>,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SceneVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceHandle<T> {
    index: usize,
    _marker: PhantomData<fn() -> T>,
}

pub type MeshNodeID = ResourceHandle<MeshNode>;
pub type MeshID = ResourceHandle<Mesh>;
pub type ShaderID = ResourceHandle<Shader>;
pub type TextureID = ResourceHandle<Texture2D>;
pub type MaterialID = ResourceHandle<Material>;

type MeshChildrenStorage = smallvec::SmallVec<[MeshNodeID; 3]>;

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

pub struct Material {
    pub material_properties: MaterialProperties,
    pub shader_ref: ShaderID,
}

impl Scene {
    pub fn load_data_from_file(&mut self, scene_file: &std::path::Path) {
        let _timer =
            timer::ScopedTimer::new(&format!("Loading scene: {}", scene_file.to_str().unwrap()));

        let (document, buffers, images) = gltf::import(scene_file).expect(&format!(
            "Failed to load scene from {}",
            scene_file.display()
        ));

        println!(
            "Loading scene: {} | Mesh count: {} | Material count: {} | Number of nodes: {}",
            scene_file.display(),
            document.meshes().len(),
            document.materials().len(),
            document.nodes().len()
        );

        let vertex_layout =
            graphics::vertex::VertexLayout::from_attribs(&Scene::VERTEX_LAYOUT_ATTRIBS);

        let mut nodes: std::vec::Vec<gltf::Node> = document
            .nodes()
            .filter(|node| node.mesh().is_some_and(|mesh| mesh.primitives().len() > 0))
            .collect();

        nodes.sort_by(|a, b| b.children().len().cmp(&a.children().len()));

        let mut discovered_nodes = HashSet::new();

        for node in nodes {
            if !discovered_nodes.insert(node.index()) {
                continue;
            }

            let gltf_mesh = node.mesh().unwrap();

            debug_assert!(
                gltf_mesh.primitives().len() == 1,
                "Multiple mesh primitives not supported"
            );

            let mut child_refs = MeshChildrenStorage::new();

            for child in node.children() {
                discovered_nodes.insert(child.index());

                debug_assert!(
                    child.children().len() == 0,
                    "Unsupported node hierarchy depth"
                );

                child_refs.push(self.add_mesh_node(
                    &child,
                    MeshChildrenStorage::new(),
                    &buffers,
                    &vertex_layout,
                ));
            }

            let _ = self.add_mesh_node(&node, child_refs, &buffers, &vertex_layout);

            self.mesh_node_roots
                .push(ResourceHandle::new(self.mesh_nodes.len() - 1));
        }
    }

    fn add_mesh_node(
        &mut self,
        node: &gltf::Node,
        children: MeshChildrenStorage,
        buffers: &[gltf::buffer::Data],
        vertex_layout: &vertex::VertexLayout,
    ) -> ResourceHandle<MeshNode> {
        let mesh = node
            .mesh()
            .expect("Filtered nodes should always have a mesh");

        println!("Loading node: {}", node.name().unwrap_or_default());

        debug_assert_eq!(mesh.primitives().len(), 1);

        let primitive = mesh.primitives().next().unwrap();

        let transform = transform_from_node(node);

        self.meshes.push(Scene::mesh_from_primitive(
            &primitive,
            buffers,
            vertex_layout,
        ));

        let mesh_handle = ResourceHandle::new(self.meshes.len() - 1);

        self.mesh_nodes.push(MeshNode::new(
            transform,
            children,
            mesh_handle,
            ResourceHandle::new(0),
        ));

        ResourceHandle::new(self.mesh_nodes.len() - 1)
    }

    fn mesh_from_primitive(
        primitive: &gltf::Primitive,
        data_buffers: &[gltf::buffer::Data],
        vertex_layout: &graphics::vertex::VertexLayout,
    ) -> Mesh {
        let reader = primitive.reader(|buffer| Some(&data_buffers[buffer.index()]));

        let positions: Vec<[f32; 3]> = reader
            .read_positions()
            .expect("Mesh contains no vertex Position data.")
            .collect();

        let mut vertex_buffer_data: Vec<SceneVertex> = positions
            .iter()
            .map(|p| SceneVertex::new(flip_z_axis(p), [0.0, 0.0, 0.0], [0.0, 0.0]))
            .collect();

        if let Some(uvs) = reader.read_tex_coords(0) {
            for (vertex, uv) in vertex_buffer_data.iter_mut().zip(uvs.into_f32()) {
                vertex.uv = uv;
            }
        } else {
            println!("Warning: Mesh has no TEXCOORD_0. Defaulting UVs to (0,0).");
        }

        if let Some(normals) = reader.read_normals() {
            for (vertex, normal) in vertex_buffer_data.iter_mut().zip(normals) {
                vertex.normal = flip_z_axis(&normal);
            }
        } else {
            println!("Warning: Mesh has no normals. Defaulting normals to (0, 0, 0).");
        }

        let mut index_buffer_data: Vec<u32> = if let Some(indices) = reader.read_indices() {
            indices.into_u32().collect()
        } else {
            println!("Mesh is non-indexed. Generating sequential indices.");
            (0..vertex_buffer_data.len() as u32).collect()
        };

        flip_triangle_winding(&mut index_buffer_data);

        let mut mesh = mesh::Mesh::new();
        mesh.upload_vertex_buffer_data(&vertex_buffer_data, vertex_layout, Usage::Static);
        mesh.upload_index_buffer_data(&index_buffer_data, Usage::Static);

        mesh
    }

    pub fn test_get_root_nodes_slice(&self) -> &[MeshNodeID] {
        self.mesh_node_roots.as_slice()
    }

    pub fn get_node(&self, id: &MeshNodeID) -> &MeshNode {
        &self.mesh_nodes[id.index]
    }

    pub fn create_test_shader() -> Shader {
        graphics::shader::Shader::compile_from_strings(
            TEST_SHADER_VERT_SOURCE,
            TEST_SHADER_FRAG_SOURCE,
        )
    }

    pub fn get_mesh(&self, id: &MeshID) -> &Mesh {
        &self.meshes[id.index]
    }

    pub fn new() -> Self {
        Self {
            meshes: Vec::<Mesh>::new(),
            shaders: Vec::<NamedShader>::new(),
            textures: Vec::<Texture2D>::new(),
            materials: Vec::<Material>::new(),
            mesh_nodes: Vec::<MeshNode>::new(),
            mesh_node_roots: Vec::<MeshNodeID>::new(),
        }
    }

    pub const VERTEX_LAYOUT_ATTRIBS: [vertex::Attrib; 3] = [
        vertex::Attrib::POSITION,
        vertex::Attrib::NORMAL,
        vertex::Attrib::UV,
    ];
}

impl SceneVertex {
    pub const fn new(position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Self {
        Self {
            position: position,
            normal: normal,
            uv: uv,
        }
    }
}

fn transform_from_node(node: &gltf::Node) -> Transform {
    Transform::from_model_matrix(Transform::model_rh_to_lh(mat4_from_gltf(
        node.transform().matrix(),
    )))
}

fn mat4_from_gltf(m: [[f32; 4]; 4]) -> Mat4 {
    Mat4::from_cols_array_2d(&m)
}

fn flip_z_axis(vector: &[f32; 3]) -> [f32; 3] {
    [vector[0], vector[1], -vector[2]]
}

fn flip_triangle_winding(indices: &mut [u32]) {
    for i in (0..indices.len()).step_by(3) {
        let tmp = indices[i];
        indices[i] = indices[i + 2];
        indices[i + 2] = tmp;
    }
}

const TEST_SHADER_VERT_SOURCE: &str = "
#version 460 core
layout (location = 0) in vec3 a_position;
layout (location = 1) in vec3 a_normal;
layout (location = 2) in vec2 a_uv;

layout(std430, binding = 0) buffer shared_data_buffer {
    mat4 camera_vp_matrix;
    mat4 camera_view_matrix;
    mat4 camera_projection_matrix;
    vec4 camera_position;
    vec4 camera_forward;
} shared_data;

uniform mat4 u_model_matrix = mat4(1.0);

out vec2 v_uv;
out vec3 v_normal;

void main() {
    vec4 position_ws = u_model_matrix * vec4(a_position, 1);
    gl_Position = shared_data.camera_vp_matrix * position_ws;

    v_uv = a_uv;
    v_normal = a_normal;
}
";

const TEST_SHADER_FRAG_SOURCE: &str = "
#version 460 core
layout (location = 0) out vec4 out_color;

in vec2 v_uv;
in vec3 v_normal;

void main() {
    out_color = vec4(v_normal, 1);
}
";

impl<T> ResourceHandle<T> {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            _marker: PhantomData,
        }
    }
}
