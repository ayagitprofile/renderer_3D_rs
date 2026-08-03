use std::collections::{HashMap, HashSet};

use glam::Mat4;
use smol_str::{SmolStr, ToSmolStr};

use super::data::{Material, MaterialID, MeshID, NamedShader, NamedTexture, Node, NodeID, ShaderID, TextureID};

use crate::{
    ShaderSource,
    graphics::{self, material_properties::MaterialProperties, mesh::Mesh, shader::Shader, texture::Texture2D, vertex},
    timer,
    transform::Transform,
};

pub struct Scene {
    nodes: Vec<Node>,
    meshes: Vec<Mesh>,
    shaders: Vec<NamedShader>,
    textures: Vec<NamedTexture>,
    materials: Vec<Material>,
    node_roots: Vec<NodeID>,
    shader_mat_props: HashMap<SmolStr, MaterialProperties>,
    default_texture_ids: DefaultTextures,
}

struct DefaultTextures {
    albedo: TextureID,
    normal: TextureID,
    metallic_roughness: TextureID,
}

impl DefaultTextures {
    pub fn iter(&self) -> impl Iterator<Item = &TextureID> {
        [&self.albedo, &self.normal, &self.metallic_roughness].into_iter()
    }

    fn new(albedo: TextureID, normal: TextureID, metallic_roughness: TextureID) -> Self {
        Self {
            albedo,
            normal,
            metallic_roughness,
        }
    }
}

const DEFAULT_SHADER_ID: ShaderID = ShaderID::new(0);

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

        let vertex_layout = graphics::vertex::VertexLayout::from_attribs(&super::data::VERTEX_LAYOUT_ATTRIBS);

        let mut nodes: std::vec::Vec<gltf::Node> = document
            .nodes()
            .filter(|node| node.mesh().is_some_and(|mesh| mesh.primitives().len() > 0))
            .collect();

        nodes.sort_by(|a, b| b.children().len().cmp(&a.children().len()));

        let mut discovered_nodes = HashSet::new();

        let mut discovered_materials = HashMap::new();

        for node in nodes {
            if !discovered_nodes.insert(node.index()) {
                continue;
            }

            let gltf_mesh = node.mesh().unwrap();

            debug_assert!(
                gltf_mesh.primitives().len() == 1,
                "Multiple mesh primitives not supported"
            );

            let mut child_refs = super::data::NodeIDStorage::new();

            for child in node.children() {
                discovered_nodes.insert(child.index());

                debug_assert!(child.children().len() == 0, "Unsupported node hierarchy depth");

                child_refs.push(self.create_node(
                    &child,
                    super::data::NodeIDStorage::new(),
                    &buffers,
                    &vertex_layout,
                    &mut discovered_materials,
                ));
            }

            let root_node_id = self.create_node(&node, child_refs, &buffers, &vertex_layout, &mut discovered_materials);

            self.add_root_node(root_node_id);
        }

        self.shader_mat_props.clear();
    }

    fn create_node(
        &mut self,
        node: &gltf::Node,
        children: super::data::NodeIDStorage,
        buffers: &[gltf::buffer::Data],
        vertex_layout: &vertex::VertexLayout,
        discovered_materials: &mut HashMap<usize, MaterialID>,
    ) -> NodeID {
        let mesh = node.mesh().expect("Filtered nodes should always have a mesh");

        debug_assert_eq!(mesh.primitives().len(), 1);

        let primitive = mesh.primitives().next().expect("Mesh has no primitives");

        let transform = transform_from_node(node);

        let mesh_id = self.add_mesh(Scene::mesh_from_primitive(&primitive, buffers, vertex_layout));

        let gltf_material = primitive.material();

        let material_id = self.resolve_imported_material(&gltf_material, discovered_materials);
        self.insert_required_default_textures(material_id);

        let node_id = self.add_node(Node::new(transform, children, mesh_id, material_id));

        node_id
    }

    fn resolve_imported_material(
        &mut self,
        gltf_material: &gltf::Material,
        discovered_materials: &mut HashMap<usize, MaterialID>,
    ) -> MaterialID {
        if let Some(gltf_material_index) = gltf_material.index() {
            if let Some(existing_material) = discovered_materials.get(&gltf_material_index) {
                println!(
                    "[Scene] Existing material found: (name: {}, id: {}), material duplication avoided\n",
                    gltf_material.name().unwrap_or_default(),
                    gltf_material_index
                );

                *existing_material
            } else {
                println!(
                    "[Scene] Found new GLTF material: (name: {}, id: {}), creating new material instance\n",
                    gltf_material.name().unwrap_or_default(),
                    gltf_material_index
                );
                let id = self.create_material_from_gltf(gltf_material);
                discovered_materials.insert(gltf_material_index, id);

                id
            }
        } else {
            println!("[Scene] Warning: nameless or default GLTF material found, creating a duplicate material for it");
            self.create_material_from_gltf(gltf_material)
        }
    }

    fn insert_required_default_textures(&mut self, material_id: MaterialID) {
        let shader_id = self.materials[material_id.index].shader_id;
        let shader = &self.shaders[shader_id.index].shader;

        for texture_resource_id in self.default_texture_ids.iter() {
            let texture_name = self.get_texture_name(*texture_resource_id);
            let location = shader.find_uniform_location(texture_name);

            if location.is_some() {
                self.materials[material_id.index].texture_ids.push(*texture_resource_id);
            }
        }
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

        let mut vertex_buffer_data: Vec<super::data::Vertex> = positions
            .iter()
            .map(|p| super::data::Vertex::new(flip_z_axis(p), [0.0, 0.0, 0.0], [0.0, 0.0]))
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

        let usage = graphics::buffer::Usage::Static;

        let mut mesh = graphics::mesh::Mesh::new();
        mesh.upload_vertex_buffer_data(&vertex_buffer_data, vertex_layout, usage);
        mesh.upload_index_buffer_data(&index_buffer_data, usage);

        mesh
    }

    fn create_material_from_gltf(&mut self, gltf_material: &gltf::Material) -> MaterialID {
        let material_name = gltf_material.name().unwrap_or_default();

        let shader_id = ShaderID::new(
            self.shaders
                .iter()
                .position(|el| el.name == material_name)
                .unwrap_or(DEFAULT_SHADER_ID.index),
        );

        let pbr = gltf_material.pbr_metallic_roughness();

        let shader_name = self.get_shader_name(shader_id);

        let mat_props = self
            .shader_mat_props
            .get(shader_name)
            .copied()
            .unwrap_or(MaterialProperties::DEFAULT);

        let texture_ids = super::data::TextureIDStorage::new();

        let material_id = self.add_material(Material::new(shader_id, &mat_props, shader_name, texture_ids));

        material_id
    }

    pub fn load_shaders(&mut self, shader_file_paths: &[std::path::PathBuf]) {
        self.shader_mat_props.reserve(shader_file_paths.len());
        self.shaders.reserve_exact(shader_file_paths.len());

        for path_buf in shader_file_paths {
            let path = path_buf.as_path();
            let shader_source = ShaderSource::load_from_file(path);

            let shader = NamedShader {
                shader: shader_source.compile(),
                name: shader_source.name().to_smolstr(),
            };

            self.shader_mat_props
                .insert(shader.name.clone(), *shader_source.mat_props());
            self.shaders.push(shader);
        }
    }

    pub fn new() -> Self {
        let (default_textures, default_texture_ids) = Scene::create_default_textures();

        let mut scene = Scene {
            meshes: Vec::<Mesh>::new(),
            shaders: Vec::<NamedShader>::new(),
            textures: default_textures,
            materials: Vec::<Material>::new(),
            nodes: Vec::<Node>::new(),
            node_roots: Vec::<NodeID>::new(),
            shader_mat_props: HashMap::<SmolStr, MaterialProperties>::new(),
            default_texture_ids: default_texture_ids,
        };

        scene.add_shader(NamedShader {
            shader: ShaderSource::load_and_compile(std::path::Path::new("assets/shaders/scene_shader.glsl")),
            name: "Default shader".to_smolstr(),
        });

        scene
    }

    fn create_default_textures() -> (Vec<NamedTexture>, DefaultTextures) {
        const TANGENT_SPACE_UP: [f32; 4] = [0f32, 0f32, 1f32, 0f32];
        const ALBEDO: [f32; 4] = [1f32, 0f32, 1f32, 1f32];
        const METALLIC_ROUGHNESS: [f32; 4] = [0f32, 0.5f32, 0f32, 1f32];

        use graphics::texture::FilteringMode;
        use graphics::texture::StorageFormat;

        let default_textures = std::vec![
            NamedTexture {
                texture: Texture2D::create_single_color_texture(
                    16,
                    16,
                    StorageFormat::RGBA,
                    &ALBEDO,
                    FilteringMode::Nearest
                ),
                name: super::textures::ALBEDO_TEXTURE_NAME.to_smolstr(),
            },
            NamedTexture {
                texture: Texture2D::create_single_color_texture(
                    16,
                    16,
                    StorageFormat::RGB16F,
                    &TANGENT_SPACE_UP,
                    FilteringMode::Nearest
                ),
                name: super::textures::NORMAL_TEXTURE_NAME.to_smolstr(),
            },
            NamedTexture {
                texture: Texture2D::create_single_color_texture(
                    16,
                    16,
                    StorageFormat::RGB,
                    &METALLIC_ROUGHNESS,
                    FilteringMode::Nearest
                ),
                name: super::textures::METALLIC_ROUGHNESS_TEXTURE_NAME.to_smolstr(),
            }
        ];

        (
            default_textures,
            DefaultTextures::new(TextureID::new(0), TextureID::new(1), TextureID::new(2)),
        )
    }
}

// resource management
impl Scene {
    pub fn add_shader(&mut self, shader: NamedShader) -> ShaderID {
        self.shaders.push(shader);
        ShaderID::new(self.shaders.len() - 1)
    }

    pub fn add_texture(&mut self, texture: NamedTexture) -> TextureID {
        self.textures.push(texture);
        TextureID::new(self.textures.len() - 1)
    }

    pub fn add_root_node(&mut self, node: NodeID) {
        self.node_roots.push(node);
    }

    pub fn add_node(&mut self, node: Node) -> NodeID {
        self.nodes.push(node);
        NodeID::new(self.nodes.len() - 1)
    }

    fn find_material_index(&self, name: &str) -> Option<usize> {
        self.materials.iter().position(|mat| mat.name == name)
    }

    pub fn add_material(&mut self, material: Material) -> MaterialID {
        self.materials.push(material);
        MaterialID::new(self.materials.len() - 1)
    }

    pub fn add_mesh(&mut self, mesh: Mesh) -> MeshID {
        self.meshes.push(mesh);
        MeshID::new(self.meshes.len() - 1)
    }

    pub fn get_mesh(&self, id: MeshID) -> &Mesh {
        &self.meshes[id.index]
    }

    pub fn get_shader(&self, id: ShaderID) -> &Shader {
        &self.shaders[id.index].shader
    }

    pub fn get_shader_name(&self, id: ShaderID) -> &str {
        self.shaders[id.index].name.as_str()
    }

    pub fn get_texture(&self, id: TextureID) -> &Texture2D {
        &self.textures[id.index].texture
    }

    pub fn get_texture_name(&self, id: TextureID) -> &str {
        &self.textures[id.index].name
    }

    pub fn get_material(&self, id: MaterialID) -> &Material {
        &self.materials[id.index]
    }

    pub fn get_node(&self, id: NodeID) -> &Node {
        &self.nodes[id.index]
    }

    pub fn root_node_iter(&self) -> std::slice::Iter<'_, super::resource_id::ResourceID<Node>> {
        self.node_roots.iter()
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
