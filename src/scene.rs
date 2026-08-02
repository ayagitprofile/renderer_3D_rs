use glam::Mat4;
use smol_str::SmolStr;

use crate::{
    graphics::{
        self,
        buffer::Usage,
        material_properties::MaterialProperties,
        mesh::{self, Mesh},
        shader::Shader,
        texture::Texture2D,
        vertex,
    }, scene_data::{Material, MaterialID, MeshChildrenStorage, MeshID, MeshNode, MeshNodeID, NamedShader, ResourceHandle, ShaderID}, shader_source::ShaderSource, timer, transform::Transform,
};
use std::{
    clone,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    vec::Vec,
};

pub struct Scene {
    meshes: Vec<Mesh>,
    shaders: Vec<NamedShader>,
    textures: Vec<Texture2D>,
    materials: Vec<Material>,
    mesh_nodes: Vec<MeshNode>,
    mesh_node_roots: Vec<MeshNodeID>,
    shader_source_data: Vec<ShaderSource>,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SceneVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl Scene {
    pub fn load_data_from_file(&mut self, scene_file: &std::path::Path) {
        let _timer = timer::ScopedTimer::start(&format!("Loading scene: {}", scene_file.to_str().unwrap()));

        let (document, buffers, images) =
            gltf::import(scene_file).expect(&format!("Failed to load scene from {}", scene_file.display()));

        println!(
            "[Scene] Loading scene: {} | Mesh count: {} | Material count: {} | Number of nodes: {}",
            scene_file.display(),
            document.meshes().len(),
            document.materials().len(),
            document.nodes().len()
        );

        let vertex_layout = graphics::vertex::VertexLayout::from_attribs(&Scene::VERTEX_LAYOUT_ATTRIBS);

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

                debug_assert!(child.children().len() == 0, "Unsupported node hierarchy depth");

                child_refs.push(self.load_mesh_node(&child, MeshChildrenStorage::new(), &buffers, &vertex_layout));
            }

            let _ = self.load_mesh_node(&node, child_refs, &buffers, &vertex_layout);

            self.mesh_node_roots
                .push(ResourceHandle::new(self.mesh_nodes.len() - 1));
        }

        self.shader_source_data.clear();
    }

    fn load_mesh_node(
        &mut self,
        node: &gltf::Node,
        children: MeshChildrenStorage,
        buffers: &[gltf::buffer::Data],
        vertex_layout: &vertex::VertexLayout,
    ) -> ResourceHandle<MeshNode> {
        let mesh = node.mesh().expect("Filtered nodes should always have a mesh");

        debug_assert_eq!(mesh.primitives().len(), 1);

        let primitive = mesh.primitives().next().unwrap();

        let transform = transform_from_node(node);

        self.meshes
            .push(Scene::mesh_from_primitive(&primitive, buffers, vertex_layout));

        let mesh_handle = ResourceHandle::new(self.meshes.len() - 1);

        let material_handle = self.create_material(primitive.material().name().unwrap_or_default());

        let mesh_node_handle = self.add_mesh_node(MeshNode::new(transform, children, mesh_handle, material_handle));

        mesh_node_handle
    }

    fn add_material(&mut self, material: Material) -> MaterialID {
        self.materials.push(material);
        MaterialID::new(self.materials.len() - 1)
    }

    fn add_mesh_node(&mut self, node: MeshNode) -> MeshNodeID {
        self.mesh_nodes.push(node);
        MeshNodeID::new(self.mesh_nodes.len() - 1)
    }

    fn create_material(&mut self, material_name: &str) -> MaterialID {
        let shader_handle = ShaderID::new(
            self.shaders
                .iter()
                .position(|el| el.name == material_name)
                .unwrap_or(0),
        );

        let shader_name = &self.get_shader(&shader_handle).name;

        let mat_props = if let Some(data) = self.shader_source_data.iter().find(|el| el.name() == shader_name) {
            *data.mat_props()
        } else {
            MaterialProperties::DEFAULT
        };

        let material_handle = self.add_material(Material::new(shader_handle, &mat_props, shader_name));

        material_handle
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
            println!("Warning: Mesh has no UV_0 channel. Defaulting UVs to (0,0).");
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

    pub fn get_shader(&self, id: &ShaderID) -> &NamedShader {
        &self.shaders[id.index]
    }

    pub fn get_material(&self, id: &MaterialID) -> &Material {
        &self.materials[id.index]
    }

    pub fn create_test_shader() -> Shader {
        ShaderSource::load_and_compile(std::path::Path::new("assets/shaders/scene_shader.glsl"))
    }

    pub fn get_mesh(&self, id: &MeshID) -> &Mesh {
        &self.meshes[id.index]
    }

    pub fn new() -> Self {
        let mut scene = Scene {
            meshes: Vec::<Mesh>::new(),
            shaders: Vec::<NamedShader>::new(),
            textures: Vec::<Texture2D>::new(),
            materials: Vec::<Material>::new(),
            mesh_nodes: Vec::<MeshNode>::new(),
            mesh_node_roots: Vec::<MeshNodeID>::new(),
            shader_source_data: Vec::<ShaderSource>::new(),
        };

        scene.shaders.push(NamedShader {
            name: "Default shader".to_string(),
            shader: Scene::create_test_shader(),
        });

        scene
    }

    pub fn load_shaders(&mut self, shader_file_paths: &[std::path::PathBuf]) {
        self.shader_source_data.reserve_exact(shader_file_paths.len());
        self.shaders.reserve_exact(shader_file_paths.len());

        for path in shader_file_paths {
            let shader_source = ShaderSource::load_from_file(path.as_path());
            let named_shader = NamedShader {
                shader: shader_source.compile(),
                name: shader_source.name().to_string(),
            };

            self.shader_source_data.push(shader_source);
            self.shaders.push(named_shader);
        }
    }

    pub const VERTEX_LAYOUT_ATTRIBS: [vertex::Attrib; 3] =
        [vertex::Attrib::POSITION, vertex::Attrib::NORMAL, vertex::Attrib::UV];

    const DEFAULT_SHADER_ID: ResourceHandle<NamedShader> = ResourceHandle::new(0);
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
    Transform::from_model_matrix(Transform::model_rh_to_lh(mat4_from_gltf(node.transform().matrix())))
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

