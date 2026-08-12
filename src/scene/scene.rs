use std::collections::{HashMap, HashSet};

use glam::Mat4;
use gltf::material::AlphaMode;
use smol_str::{SmolStr, ToSmolStr};

use super::{
    data::{Material, MaterialID, MeshID, NamedShader, NamedTexture, Node, NodeID, ShaderID, TextureID},
    light_data_buffer::LightDataStorage,
};

use crate::{
    graphics::{
        self,
        material_properties::MaterialProperties,
        mesh::Mesh,
        shader::Shader,
        texture::{Cubemap, Texture2D},
        utility::CubemapSide,
        vertex,
    },
    scene::{data::AABB, light::LightType},
    shader_source::ShaderSource,
    timer,
    transform::Transform,
};

#[derive(Clone)]
pub struct ShaderMaterialMapping {
    pub shader_name: String,
    pub associated_materials: Vec<String>,
}

pub struct CustomShaders<'a> {
    pub shader_file_paths: &'a [std::path::PathBuf],
    pub mapping: &'a [ShaderMaterialMapping],
}

impl ShaderMaterialMapping {
    pub fn new(shader_name: &str, associated_materials: &[&str]) -> Self {
        Self {
            shader_name: shader_name.to_string(),
            associated_materials: associated_materials.iter().map(|val| val.to_string()).collect(),
        }
    }
}

pub struct Scene {
    nodes: Vec<Node>,
    meshes: Vec<Mesh>,
    shaders: Vec<NamedShader>,
    textures: Vec<NamedTexture>,
    materials: Vec<Material>,
    node_roots: Vec<NodeID>,
    shader_mat_props: HashMap<SmolStr, MaterialProperties>,
    default_texture_ids: DefaultTextures,
    shader_name_mapping: Vec<ShaderMaterialMapping>,

    default_opaque_shader: (ShaderID, MaterialProperties),
    default_transparent_shader: (ShaderID, MaterialProperties),

    cubemap: Cubemap,

    pub lights: LightDataStorage,
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

impl Scene {
    const SPECIAL_CUBEMAP_TEXTURE_CONTAINER_OBJECT_NAME: &str = "cubemap_texture_set";

    fn load_cubemap(cubemap_container_node: &gltf::Node, image_storage: &Vec<gltf::image::Data>) -> Option<Cubemap> {
        let mut cubemap_data = std::collections::HashMap::new();

        for node in cubemap_container_node.children().filter(|el| el.mesh().is_some()) {
            if let Some(primitive) = node.mesh().unwrap().primitives().last() {
                if let Some(texture_info) = primitive.material().pbr_metallic_roughness().base_color_texture() {
                    if let Some(cubemap_side) =
                        graphics::utility::CubemapSide::from_str(primitive.material().name().unwrap_or_default())
                    {
                        cubemap_data.insert(cubemap_side, texture_info.texture().source().index());
                    }
                }
            }
        }

        if cubemap_data.len() != 6 {
            println!(
                "[Scene] Failed to load cubemap, not enough textures provided: {}",
                cubemap_data.len()
            );
            return None;
        }

        if cubemap_data.iter().all(|el| {
            let reference_data = &image_storage[cubemap_data[&CubemapSide::Left]];

            let data = &image_storage[*el.1];
            data.format == reference_data.format
                && data.height == reference_data.height
                && data.width == reference_data.width
        }) == false
        {
            println!("[Scene] Failed to load cubemap, textures have different metadata");
            return None;
        }

        let data_channels = match image_storage[cubemap_data[&CubemapSide::Left]].format {
            gltf::image::Format::R8 => 1,
            gltf::image::Format::R8G8 => 2,
            gltf::image::Format::R8G8B8 => 3,
            gltf::image::Format::R8G8B8A8 => 4,

            _ => {
                println!(
                    "[Scene] Failed to load cubemap, unsupported data format: {:?}",
                    image_storage[cubemap_data[&CubemapSide::Left]].format
                );

                return None;
            }
        };

        let (width, height) = (
            image_storage[cubemap_data[&CubemapSide::Left]].width,
            image_storage[cubemap_data[&CubemapSide::Left]].height,
        );

        Some(graphics::texture::Cubemap::load_from_memory(
            width,
            height,
            data_channels,
            graphics::texture::StorageFormat::RGB,
            graphics::texture::FilteringMode::Trilinear,
            image_storage[cubemap_data[&CubemapSide::Left]].pixels.as_ptr(),
            image_storage[cubemap_data[&CubemapSide::Right]].pixels.as_ptr(),
            image_storage[cubemap_data[&CubemapSide::Top]].pixels.as_ptr(),
            image_storage[cubemap_data[&CubemapSide::Bottom]].pixels.as_ptr(),
            image_storage[cubemap_data[&CubemapSide::Front]].pixels.as_ptr(),
            image_storage[cubemap_data[&CubemapSide::Back]].pixels.as_ptr(),
        ))
    }

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
            .filter(|node| node.mesh().is_some_and(|mesh| mesh.primitives().len() > 0) || node.light().is_some())
            .collect();

        nodes.sort_by(|a, b| b.children().len().cmp(&a.children().len()));

        let mut discovered_nodes = HashSet::new();

        let mut discovered_materials = HashMap::new();

        let mut lights = Vec::new();

        for node in nodes {
            if !discovered_nodes.insert(node.index()) {
                continue;
            }

            if let Some(light) = node.light() {
                use gltf::khr_lights_punctual::Kind as LightKind;

                let node_transform = transform_from_node(&node);

                let light_data = match light.kind() {
                    LightKind::Directional => super::light::LightData::new_directional_light(
                        node_transform.forward().to_array(),
                        light.color(),
                        light.intensity(),
                    ),
                    LightKind::Point => super::light::LightData::new_point_light(
                        node_transform.position().to_array(),
                        light.color(),
                        light.intensity(),
                        light.range().unwrap_or(1000f32),
                    ),
                    LightKind::Spot {
                        inner_cone_angle,
                        outer_cone_angle,
                    } => super::light::LightData::new(
                        LightType::Spot,
                        node_transform.position().to_array(),
                        node_transform.forward().to_array(),
                        light.color(),
                        light.intensity(),
                        1f32,
                        1f32,
                        1f32,
                        light.range().unwrap_or(1000f32),
                        inner_cone_angle.cos(),
                        outer_cone_angle.cos(),
                    ),
                };

                lights.push(light_data);
                continue;
            }

            if node.name().unwrap_or_default() == Scene::SPECIAL_CUBEMAP_TEXTURE_CONTAINER_OBJECT_NAME {
                for child in node.children() {
                    discovered_nodes.insert(child.index());
                }

                if let Some(cubemap) = Scene::load_cubemap(&node, &images) {
                    self.cubemap = cubemap;
                }

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
                    &images,
                ));
            }

            let root_node_id = self.create_node(
                &node,
                child_refs,
                &buffers,
                &vertex_layout,
                &mut discovered_materials,
                &images,
            );

            self.add_root_node(root_node_id);
        }

        self.lights = LightDataStorage::new(&lights);
        self.shader_mat_props = HashMap::new();
        self.shader_name_mapping = Vec::new();
        ShaderSource::clear_cache();
    }

    fn create_node(
        &mut self,
        node: &gltf::Node,
        children: super::data::NodeIDStorage,
        buffers: &[gltf::buffer::Data],
        vertex_layout: &vertex::VertexLayout,
        discovered_materials: &mut HashMap<usize, MaterialID>,
        images: &Vec<gltf::image::Data>,
    ) -> NodeID {
        let mesh = node.mesh().expect("Filtered nodes should always have a mesh");

        debug_assert_eq!(mesh.primitives().len(), 1);

        let primitive = mesh.primitives().next().expect("Mesh has no primitives");

        let transform = transform_from_node(node);

        let mesh_id = self.add_mesh(Scene::mesh_from_primitive(&primitive, buffers, vertex_layout));

        let gltf_material = primitive.material();

        let material_id = self.resolve_imported_material(&gltf_material, discovered_materials, images);

        let node_id = self.add_node(Node::new(
            transform,
            children,
            mesh_id,
            material_id,
            Scene::aabb_from_gltf_bounding_box(&primitive.bounding_box()),
        ));

        node_id
    }

    fn aabb_from_gltf_bounding_box(bb: &gltf::mesh::BoundingBox) -> AABB {
        // to prevent flat planes from being incorrectly culled
        const MIN_AABB_AXIS_SIZE: f32 = 0.1f32;

        let mut min = bb.min;
        let mut max = bb.max;

        let dif_x = max[0] - min[0];
        let dif_y = max[1] - min[1];
        let dif_z = max[2] - min[2];

        if dif_x < MIN_AABB_AXIS_SIZE {
            min[0] += MIN_AABB_AXIS_SIZE * 0.5f32;
            max[0] += MIN_AABB_AXIS_SIZE * 0.5f32;
        }

        if dif_y < MIN_AABB_AXIS_SIZE {
            min[1] += MIN_AABB_AXIS_SIZE * 0.5f32;
            max[1] += MIN_AABB_AXIS_SIZE * 0.5f32;
        }

        if dif_z < MIN_AABB_AXIS_SIZE {
            min[2] += MIN_AABB_AXIS_SIZE * 0.5f32;
            max[2] += MIN_AABB_AXIS_SIZE * 0.5f32;
        }

        let aabb_lhs = AABB::new([min[0], min[1], -max[2]], [max[0], max[1], -min[2]]);

        aabb_lhs
    }

    fn resolve_imported_material(
        &mut self,
        gltf_material: &gltf::Material,
        discovered_materials: &mut HashMap<usize, MaterialID>,
        images: &Vec<gltf::image::Data>,
    ) -> MaterialID {
        if let Some(gltf_material_index) = gltf_material.index() {
            if let Some(existing_material) = discovered_materials.get(&gltf_material_index) {
                *existing_material
            } else {
                let id = self.create_material_from_gltf(gltf_material, images);
                discovered_materials.insert(gltf_material_index, id);

                id
            }
        } else {
            self.create_material_from_gltf(gltf_material, images)
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
            .map(|p| super::data::Vertex::new(flip_z_axis(p), [0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0], [0.0, 0.0]))
            .collect();

        if let Some(uvs) = reader.read_tex_coords(0) {
            for (vertex, uv) in vertex_buffer_data.iter_mut().zip(uvs.into_f32()) {
                vertex.uv = uv;
            }
        } else {
            println!("Error: Mesh has no UV_0 channel. Defaulting UVs to (0,0).");
        }

        if let Some(normals) = reader.read_normals() {
            for (vertex, normal) in vertex_buffer_data.iter_mut().zip(normals) {
                vertex.normal = flip_z_axis(&normal);
            }
        } else {
            println!("Error: Mesh has no normals. Defaulting normals to (0, 0, 0).");
        }

        if let Some(tangents) = reader.read_tangents() {
            for (vertex, tangent) in vertex_buffer_data.iter_mut().zip(tangents) {
                vertex.tangent_and_handedness = tangent;
            }
        } else {
            println!("Error: Mesh has no tangents. Defaulting tangents to (0, 0, 1).");
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

    fn create_material_from_gltf(
        &mut self,
        gltf_material: &gltf::Material,
        images: &Vec<gltf::image::Data>,
    ) -> MaterialID {
        let material_name = gltf_material.name().unwrap_or_default().to_string();

        let (default_scene_shader, default_mat_props) = if gltf_material.alpha_mode() == AlphaMode::Opaque {
            self.default_opaque_shader
        } else {
            self.default_transparent_shader
        };

        let shader_id = if let Some(mapping) = self
            .shader_name_mapping
            .iter()
            .find(|mapping| mapping.associated_materials.contains(&material_name))
        {
            ShaderID::new(
                self.shaders
                    .iter()
                    .position(|shader| shader.name == mapping.shader_name)
                    .unwrap_or(default_scene_shader.index),
            )
        } else {
            default_scene_shader
        };

        let shader_name = self.get_shader_name(shader_id);

        let mut mat_props = self
            .shader_mat_props
            .get(shader_name)
            .copied()
            .unwrap_or(default_mat_props);

        let texture_ids = super::data::TextureIDStorage::new();

        // if shader for this material is custom, dont assign properties of gltf material
        if shader_id == self.default_opaque_shader.0 || shader_id == self.default_transparent_shader.0 {
            mat_props.surface_type = match gltf_material.alpha_mode() {
                gltf::material::AlphaMode::Mask | gltf::material::AlphaMode::Blend => {
                    graphics::material_properties::SurfaceType::Transparent
                }
                gltf::material::AlphaMode::Opaque => graphics::material_properties::SurfaceType::Opaque,
            };

            mat_props.cull_mode = if gltf_material.double_sided() {
                graphics::material_properties::CullMode::Disabled
            } else {
                graphics::material_properties::CullMode::Back
            };
        }

        let material_id = self.add_material(Material::new(shader_id, &mat_props, shader_name, texture_ids));

        self.insert_required_default_textures(material_id);

        let shader = self.get_shader(shader_id);

        let pbr = gltf_material.pbr_metallic_roughness();

        let mut added_textures = std::vec::Vec::new();

        if shader
            .find_uniform_location(super::textures::ALBEDO_TEXTURE_NAME)
            .is_some()
            && let Some(texture) = pbr.base_color_texture()
        {
            let gltf_image = &images[texture.texture().source().index()];

            let texture = NamedTexture {
                texture: Scene::texture_from_gltf_image(
                    gltf_image,
                    graphics::texture::StorageFormat::RGBA,
                    graphics::texture::FilteringMode::AnisotropicX16,
                ),
                name: super::textures::ALBEDO_TEXTURE_NAME.to_smolstr(),
            };

            added_textures.push(texture);
        }
        if shader
            .find_uniform_location(super::textures::NORMAL_TEXTURE_NAME)
            .is_some()
            && let Some(texture_info) = gltf_material.normal_texture()
        {
            let gltf_image = &images[texture_info.texture().source().index()];

            let texture = NamedTexture {
                texture: Scene::texture_from_gltf_image(
                    gltf_image,
                    graphics::texture::StorageFormat::RGB,
                    graphics::texture::FilteringMode::AnisotropicX16,
                ),
                name: super::textures::NORMAL_TEXTURE_NAME.to_smolstr(),
            };

            added_textures.push(texture);
        }
        if shader
            .find_uniform_location(super::textures::METALLIC_ROUGHNESS_TEXTURE_NAME)
            .is_some()
            && let Some(texture_info) = pbr.metallic_roughness_texture()
        {
            let gltf_image = &images[texture_info.texture().source().index()];

            let texture = NamedTexture {
                texture: Scene::texture_from_gltf_image(
                    gltf_image,
                    graphics::texture::StorageFormat::RGB,
                    graphics::texture::FilteringMode::AnisotropicX16,
                ),
                name: super::textures::METALLIC_ROUGHNESS_TEXTURE_NAME.to_smolstr(),
            };

            added_textures.push(texture);
        }

        for texture in added_textures {
            let id = self.add_texture(texture);
            self.materials[material_id.index].texture_ids.push(id);
        }

        material_id
    }

    fn load_custom_shaders(&mut self, shader_file_paths: &[std::path::PathBuf], mapping: &[ShaderMaterialMapping]) {
        self.shader_name_mapping = mapping.to_vec();
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

    pub fn cubemap(&self) -> &Cubemap {
        &self.cubemap
    }

    pub fn new(custom_shaders: Option<CustomShaders>) -> Self {
        let (default_textures, default_texture_ids) = Scene::create_default_textures();

        let scene_shader_source =
            ShaderSource::load_from_file(std::path::Path::new("assets/shaders/scene_shader.glsl"));

        let mut transparent_mat_props = *scene_shader_source.mat_props();

        transparent_mat_props.surface_type = graphics::material_properties::SurfaceType::Transparent;
        transparent_mat_props.depth_writing_enabled = false;
        transparent_mat_props.depth_test_mode = graphics::material_properties::DepthTestMode::LessEqual;

        let data_ptr = super::textures::EMO_TEXTURE_DATA.as_ptr() as *const u8;

        let cubemap = graphics::texture::Cubemap::load_from_memory(
            2,
            2,
            4,
            graphics::texture::StorageFormat::RGB,
            graphics::texture::FilteringMode::Nearest,
            data_ptr,
            data_ptr,
            data_ptr,
            data_ptr,
            data_ptr,
            data_ptr,
        );

        let mut scene = Scene {
            meshes: Vec::<Mesh>::new(),
            shaders: Vec::<NamedShader>::new(),
            textures: default_textures,
            materials: Vec::<Material>::new(),
            nodes: Vec::<Node>::new(),
            node_roots: Vec::<NodeID>::new(),
            shader_mat_props: HashMap::<SmolStr, MaterialProperties>::new(),
            default_texture_ids: default_texture_ids,
            shader_name_mapping: Vec::new(),
            default_opaque_shader: (ShaderID::new(0), *scene_shader_source.mat_props()),
            default_transparent_shader: (ShaderID::new(1), transparent_mat_props),
            cubemap,
            lights: LightDataStorage::new(&[]),
        };

        scene.default_opaque_shader.0 = scene.add_shader(NamedShader {
            shader: scene_shader_source.compile(),
            name: "Default opaque scene shader".to_smolstr(),
        });

        scene.default_transparent_shader.0 = scene.add_shader(NamedShader {
            shader: scene_shader_source.compile(),
            name: "Default transparent scene shader".to_smolstr(),
        });

        if let Some(shaders) = custom_shaders {
            scene.load_custom_shaders(shaders.shader_file_paths, shaders.mapping);
        }

        scene
    }

    fn create_default_textures() -> (Vec<NamedTexture>, DefaultTextures) {
        const TANGENT_SPACE_UP: [f32; 4] = [0f32, 0f32, 1f32, 0f32];
        const METALLIC_ROUGHNESS: [f32; 4] = [0f32, 0.5f32, 0f32, 1f32];

        use graphics::texture::FilteringMode;
        use graphics::texture::StorageFormat;

        let default_textures = std::vec![
            NamedTexture {
                texture: Texture2D::create_texture_from_memory(
                    2,
                    2,
                    StorageFormat::SRGBA,
                    FilteringMode::Nearest,
                    4,
                    super::textures::EMO_TEXTURE_DATA.as_ptr() as *const std::ffi::c_void
                ),
                name: super::textures::ALBEDO_TEXTURE_NAME.to_smolstr(),
            },
            NamedTexture {
                texture: Texture2D::create_single_color_texture(
                    16,
                    16,
                    StorageFormat::RGB,
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

    fn texture_from_gltf_image(
        image: &gltf::image::Data,
        storage_format: graphics::texture::StorageFormat,
        filtering_mode: graphics::texture::FilteringMode,
    ) -> Texture2D {
        use gltf::image::Format;

        let channels = match image.format {
            Format::R8 => 1,
            Format::R8G8 => 2,
            Format::R8G8B8 => 3,
            Format::R8G8B8A8 => 4,
            _ => panic!("Unsupported GLTF image format"),
        };

        let (width, height) = (image.width as i32, image.height as i32);

        let data = image.pixels.as_ptr() as *const std::ffi::c_void;

        let texture = graphics::texture::Texture2D::create_texture_from_memory(
            width,
            height,
            storage_format,
            filtering_mode,
            channels,
            data,
        );

        texture
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

    pub fn get_node_mut(&mut self, id: NodeID) -> &mut Node {
        &mut self.nodes[id.index]
    }

    pub fn root_node_iter(&self) -> std::slice::Iter<'_, super::resource_id::ResourceID<Node>> {
        self.node_roots.iter()
    }

    pub fn root_nodes(&self) -> &[NodeID] {
        self.node_roots.as_slice()
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
