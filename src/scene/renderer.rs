use glam::{Mat3, Mat4, Vec3, Vec4, Vec4Swizzles};

use super::{data, scene::Scene};
use crate::ambient_occlusion::AmbientOcclusion;
use crate::camera::Camera;
use crate::graphics::buffer::GraphicsBuffer;
use crate::graphics::framebuffer::Framebuffer;
use crate::graphics::material_properties::{MaterialProperties, ShadowCasting, SurfaceType};
use crate::graphics::mesh::Mesh;
use crate::graphics::shader::Shader;
use crate::graphics::texture::{self, Texture, Texture2D};
use crate::scene::light::LightType;
use crate::scene::skybox::Skybox;
use crate::scene::value_range::{self, ValueRange};
use crate::shader_source::ShaderSource;
use crate::transform::Transform;
use crate::{ambient_occlusion, gl, graphics, timer};

const SHADER_MODEL_MATRIX_UNIFORM_NAME: &str = "u_model_matrix";

pub struct Renderer {
    current_mat_props: MaterialProperties,

    opaque_object_draw_call_data: Vec<DrawCallData>,
    transparent_object_draw_call_data: Vec<DrawCallData>,

    depth_prepass_shader: (Shader, MaterialProperties),
    skybox: Skybox,

    render_targets: RenderTargets,

    ambient_occlusion: ambient_occlusion::AmbientOcclusion,

    post_process_fs_quad: graphics::fullscreen_quad::FullscreenQuad,

    post_process_config: PostProcessingConfig,

    post_process_config_buffer: GraphicsBuffer,

    shadow_casters: Vec<DrawCallData>,
    shadow_caster_ls_matrix: Option<Mat4>,
    shadow_caster_shader: Shader,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct GPUPostProcessConfig {
    pub chromatic_abberation: f32,
    pub vignette: f32,
}

impl GPUPostProcessConfig {
    fn from_cpu_config(config: &PostProcessingConfig) -> Self {
        Self {
            chromatic_abberation: config.chromatic_abberation.value(),
            vignette: config.vignette.value(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct PostProcessingConfig {
    pub chromatic_abberation: value_range::ValueRange,
    pub vignette: value_range::ValueRange,
}

struct RenderTargets {
    render_framebuffer: Framebuffer,
    shadow_framebuffer: Framebuffer,
}

#[derive(Clone, Copy)]
struct DrawCallData {
    object_to_world: Mat4,
    world_center: Vec3,
    node_id: data::NodeID,
}

struct AABBVec3 {
    min: Vec3,
    max: Vec3,
}

impl AABBVec3 {
    fn from_scene_aabb(aabb: &data::AABB) -> Self {
        Self {
            min: Vec3::from_array(aabb.min),
            max: Vec3::from_array(aabb.max),
        }
    }

    fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }
}

fn clip_space_to_world_space(position_cs: Vec3, inverse_view_projection: &Mat4) -> Vec3 {
    let p = inverse_view_projection * Vec4::new(position_cs.x, position_cs.y, position_cs.z, 1.0f32);
    p.xyz() / p.w
}

fn camera_frustum_corners_ws(camera_vp_matrix: &Mat4) -> [Vec3; 8] {
    let inverse_vp = camera_vp_matrix.inverse();

    #[rustfmt::skip]
    let corners = [
        clip_space_to_world_space(Vec3::new(-1f32, -1f32, -1f32), &inverse_vp), // near BL
        clip_space_to_world_space(Vec3::new( 1f32, -1f32, -1f32), &inverse_vp), // near BR
        clip_space_to_world_space(Vec3::new(-1f32,  1f32, -1f32), &inverse_vp), // near TL
        clip_space_to_world_space(Vec3::new( 1f32,  1f32, -1f32), &inverse_vp), // near TR

        clip_space_to_world_space(Vec3::new(-1f32, -1f32,  1f32), &inverse_vp), // far BL
        clip_space_to_world_space(Vec3::new( 1f32, -1f32,  1f32), &inverse_vp), // far BR
        clip_space_to_world_space(Vec3::new(-1f32,  1f32,  1f32), &inverse_vp), // far TL
        clip_space_to_world_space(Vec3::new( 1f32,  1f32,  1f32), &inverse_vp), // far TR
    ];

    corners
}

struct Plane {
    normal: glam::Vec3,
    distance: f32,
}

struct CameraFrustum {
    planes: [Plane; 6],
}

pub struct RenderingStats {
    pub total_objects: u32,
    pub visible_objects: u32,
    pub frame_data_preparation_time: std::time::Duration,
}

impl Plane {
    fn distance_to_point(&self, p: Vec3) -> f32 {
        self.normal.dot(p) + self.distance
    }

    fn from_vec4(v: Vec4) -> Self {
        let normal = v.truncate();
        let inv_len = 1f32 / normal.length();

        Self {
            normal: normal * inv_len,
            distance: v.w * inv_len,
        }
    }
}

impl CameraFrustum {
    pub fn intersects_aabb(&self, aabb: &AABBVec3) -> bool {
        for plane in &self.planes {
            let mask = plane.normal.cmpge(Vec3::ZERO);
            let p = Vec3::select(mask, aabb.max, aabb.min);

            if plane.normal.dot(p) + plane.distance < 0.0 {
                return false;
            }
        }

        true
    }

    pub fn from_view_projection_matrix(view_proj: Mat4) -> Self {
        let m = view_proj.transpose();

        Self {
            planes: [
                Plane::from_vec4(m.w_axis + m.x_axis), // Left
                Plane::from_vec4(m.w_axis - m.x_axis), // Right
                Plane::from_vec4(m.w_axis + m.y_axis), // Bottom
                Plane::from_vec4(m.w_axis - m.y_axis), // Top
                Plane::from_vec4(m.w_axis + m.z_axis), // Near
                Plane::from_vec4(m.w_axis - m.z_axis), // Far
            ],
        }
    }
}

impl DrawCallData {
    fn new(node_id: data::NodeID, object_to_world: &Mat4, world_center: Vec3) -> Self {
        Self {
            node_id,
            object_to_world: *object_to_world,
            world_center,
        }
    }
}

/// transform AABB, and get world center for sorting
fn transform_aabb(aabb: AABBVec3, world: &Mat4) -> (AABBVec3, Vec3) {
    let center = (aabb.min + aabb.max) * 0.5;
    let extent = (aabb.max - aabb.min) * 0.5;

    let world_center = world.transform_point3(center);

    let world_extent = Mat3::from_mat4(*world).abs() * extent;

    (
        AABBVec3 {
            min: world_center - world_extent,
            max: world_center + world_extent,
        },
        world_center,
    )
}

fn get_light_direction_axes(light_forward: Vec3) -> (Vec3, Vec3, Vec3) {
    let forward = light_forward.normalize();

    let up_ref = if Transform::UP.dot(forward).abs() > 0.99f32 {
        Transform::RIGHT
    } else {
        Transform::UP
    };

    let right = up_ref.cross(forward).normalize();
    let up = forward.cross(right);

    (right, up, forward)
}

fn center_of_points(points: &[Vec3]) -> Vec3 {
    let mut center = Vec3::ZERO;

    for p in points {
        center += p;
    }

    center / points.len() as f32
}

impl Renderer {
    pub fn post_processing_config_mut(&mut self) -> &mut PostProcessingConfig {
        &mut self.post_process_config
    }

    pub fn prepare_rendering_data(&mut self, scene: &Scene, camera: &Camera) -> RenderingStats {
        let timer = timer::Timer::start("");

        self.opaque_object_draw_call_data.clear();
        self.transparent_object_draw_call_data.clear();
        self.shadow_casters.clear();

        let camera_frustum =
            CameraFrustum::from_view_projection_matrix(camera.projection_matrix() * camera.view_matrix());

        if let Some(directinal_light) = scene
            .lights
            .lights()
            .iter()
            .find(|light| light.type_of_light == LightType::Directional)
        {
            let light_forward = Vec3::from_array(directinal_light.direction).normalize();

            let (width, height) = (10f32, 10f32);
            let (half_width, half_height) = (width * 0.5f32, height * 0.5f32);

            let view_mat = Camera::calculate_view_matrix(light_forward * -10f32, light_forward);
            let proj_mat =
                glam::Mat4::orthographic_rh_gl(-half_width, half_width, -half_height, half_height, 0.1f32, 100f32);

            self.shadow_caster_ls_matrix = Some(proj_mat * view_mat);
        } else {
            self.shadow_caster_ls_matrix = None;
        }

        for root_node_id in scene.root_node_iter() {
            let root_node = scene.get_node(*root_node_id);

            let object_to_world = *root_node.transform.model_matrix();

            let (aabb, center) = transform_aabb(AABBVec3::from_scene_aabb(&root_node.bounding_box), &object_to_world);

            let material = scene.get_material(root_node.material_id);

            let draw_call_data = DrawCallData::new(*root_node_id, &object_to_world, center);

            // || true is for testing only
            if material.material_properties.shadow_casting != ShadowCasting::Disabled || true {
                self.shadow_casters.push(draw_call_data);
            }

            if camera_frustum.intersects_aabb(&aabb) {
                if material.material_properties.surface_type == SurfaceType::Opaque {
                    self.opaque_object_draw_call_data.push(draw_call_data);
                } else {
                    self.transparent_object_draw_call_data.push(draw_call_data);
                }
            }

            for child_id in root_node.children_iter() {
                let child_node = scene.get_node(*child_id);
                let child_object_to_world = root_node.transform.model_matrix() * child_node.transform.model_matrix();

                let (child_aabb, child_center) = transform_aabb(
                    AABBVec3::from_scene_aabb(&child_node.bounding_box),
                    &child_object_to_world,
                );

                if camera_frustum.intersects_aabb(&child_aabb) {
                    let child_material = scene.get_material(child_node.material_id);
                    if child_material.material_properties.surface_type == SurfaceType::Opaque {
                        self.opaque_object_draw_call_data.push(DrawCallData::new(
                            *child_id,
                            &child_object_to_world,
                            child_center,
                        ));
                    } else {
                        self.transparent_object_draw_call_data.push(DrawCallData::new(
                            *child_id,
                            &child_object_to_world,
                            child_center,
                        ));
                    }
                }
            }
        }

        let camera_position = camera.transform.position();

        self.opaque_object_draw_call_data.sort_by(|a, b| {
            let a_position = a.world_center;
            let a_distance_to_camera = camera_position.distance_squared(a_position);

            let b_position = b.world_center;
            let b_distance_to_camera = camera_position.distance_squared(b_position);

            a_distance_to_camera.total_cmp(&b_distance_to_camera)
        });

        self.transparent_object_draw_call_data.sort_by(|b, a| {
            let a_position = a.world_center;
            let a_distance_to_camera = camera_position.distance_squared(a_position);

            let b_position = b.world_center;
            let b_distance_to_camera = camera_position.distance_squared(b_position);

            a_distance_to_camera.total_cmp(&b_distance_to_camera)
        });

        RenderingStats {
            total_objects: scene.nodes().len() as u32,
            visible_objects: (self.opaque_object_draw_call_data.len() + self.transparent_object_draw_call_data.len())
                as u32,
            frame_data_preparation_time: timer.elapsed(),
        }
    }

    fn calculate_ssao_resolution(resolution: (u32, u32)) -> (u32, u32) {
        // (resolution.0 * 3 / 4, resolution.1 * 3 / 4)
        (resolution.0 / 2, resolution.1 / 2)
    }

    pub fn resize(&mut self, resolution: (u32, u32)) {
        println!("Renderer.resize called");

        self.render_targets = RenderTargets::new(resolution);
        self.ambient_occlusion.resize(resolution);
    }

    pub fn new(resolution: (u32, u32), scene: &Scene) -> Self {
        let depth_prepass_source =
            ShaderSource::load_from_file(std::path::Path::new("assets/shaders/depth_prepass_shader.glsl"));

        let render_targets = RenderTargets::new(resolution);

        let (ao_resolution_x, ao_resolution_y) = Renderer::calculate_ssao_resolution(resolution);

        let ao = ambient_occlusion::AmbientOcclusion::new(ao_resolution_x, ao_resolution_y);
        ao.set_input_textures(&render_targets.depth_texture(), &render_targets.normal_texture());

        let fs_quad = graphics::fullscreen_quad::FullscreenQuad::new(std::path::Path::new(
            "assets/shaders/post_process_shader.glsl",
        ));

        graphics::utility::try_set_bindless_texture(
            &fs_quad.shader,
            super::textures::FRAMEBUFFER_COLOR_TEXTURE,
            render_targets.color_texture().bindless_handle(),
        );

        let pp_config = PostProcessingConfig {
            chromatic_abberation: ValueRange::new(0.005f32, 0f32, 0.1f32),
            vignette: ValueRange::new(0.05f32, 0f32, 1f32),
        };

        let mut post_process_buffer = GraphicsBuffer::new();

        post_process_buffer.allocate(&[pp_config], graphics::buffer::Usage::Dynamic);

        post_process_buffer.set_binding(graphics::buffer::BindingTarget::UniformBuffer, 1);

        let skybox = Skybox::new();

        skybox.shader.map_bindless_texture(
            skybox
                .shader
                .find_uniform_location(super::textures::CUBEMAP_TEXTURE)
                .unwrap(),
            scene.cubemap().bindless_handle(),
        );

        let renderer = Renderer {
            current_mat_props: MaterialProperties::DEFAULT,

            opaque_object_draw_call_data: Vec::with_capacity(128),
            transparent_object_draw_call_data: Vec::with_capacity(16),
            shadow_casters: Vec::with_capacity(32),

            depth_prepass_shader: (depth_prepass_source.compile(), *depth_prepass_source.mat_props()),
            skybox,
            render_targets: render_targets,
            ambient_occlusion: ao,
            post_process_fs_quad: fs_quad,
            post_process_config: pp_config,
            post_process_config_buffer: post_process_buffer,

            shadow_caster_ls_matrix: None,

            shadow_caster_shader: ShaderSource::load_from_file(std::path::Path::new(
                "assets/shaders/scene_shadow_caster_shader.glsl",
            ))
            .compile(),
        };

        renderer
    }

    pub fn render_shadow_pass(&self, scene: &Scene) {
        if self.shadow_caster_ls_matrix.is_none() {
            return;
        }

        self.render_targets.shadow_framebuffer.bind();
        self.render_targets.shadow_framebuffer.clear_depth_attachment();

        let shader = &self.shadow_caster_shader;

        shader.bind();

        graphics::utility::set_cull_mode(graphics::material_properties::CullMode::Disabled);
        graphics::utility::set_depth_test_mode(graphics::material_properties::DepthTestMode::LessEqual);
        graphics::utility::set_depth_writing(true);

        unsafe {
            gl::Disable(gl::BLEND);
        }

        let model_matrix_uniform_location = shader.find_uniform_location("u_model_matrix").unwrap();

        // let vp_matrix = shadow_camera.projection_matrix() * shadow_camera.view_matrix();

        shader.set_uniform_mat4(
            shader.find_uniform_location("u_light_vp_matrix").unwrap(),
            &self.shadow_caster_ls_matrix.unwrap().to_cols_array(),
        );

        for data in self.shadow_casters.iter() {
            let node = scene.get_node(data.node_id);
            let mesh = scene.get_mesh(node.mesh_id);

            mesh.vao().bind();

            shader.set_uniform_mat4(model_matrix_uniform_location, &data.object_to_world.to_cols_array());

            unsafe {
                gl::DrawElements(
                    gl::TRIANGLES,
                    mesh.index_count(),
                    mesh.index_format().to_gl_format(),
                    std::ptr::null(),
                );
            }
        }
    }

    pub fn render_depth_prepass(&self, scene: &Scene) {
        self.render_targets.render_framebuffer.bind();
        self.render_targets.render_framebuffer.clear_depth_attachment();

        self.render_targets
            .render_framebuffer
            .set_active_render_target(RenderTargets::NORMAL_TEXTURE_ATTACHMENT_INDEX);

        let shader = &self.depth_prepass_shader.0;
        shader.bind();

        graphics::utility::apply_mat_props(&self.depth_prepass_shader.1);

        let model_matrix_uniform_location = shader.find_uniform_location("u_model_matrix").unwrap();

        unsafe {
            gl::Disable(gl::BLEND);
        }

        for data in self.opaque_object_draw_call_data.iter() {
            let node = scene.get_node(data.node_id);
            let mesh = scene.get_mesh(node.mesh_id);

            mesh.vao().bind();

            shader.set_uniform_mat4(model_matrix_uniform_location, &data.object_to_world.to_cols_array());

            unsafe {
                gl::DrawElements(
                    gl::TRIANGLES,
                    mesh.index_count(),
                    mesh.index_format().to_gl_format(),
                    std::ptr::null(),
                );
            }
        }
    }

    pub fn render_forward_lighting(&mut self, scene: &Scene) {
        self.render_targets.render_framebuffer.bind();

        self.render_targets
            .render_framebuffer
            .set_active_render_target(RenderTargets::COLOR_TEXTURE_ATTACHMENT_INDEX);

        for data in self.opaque_object_draw_call_data.iter() {
            self.render_node(scene, data.node_id, &data.object_to_world);
        }

        self.render_mesh(&self.skybox.mesh, &self.skybox.shader, &self.skybox.mat_props);

        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        for data in self.transparent_object_draw_call_data.iter() {
            self.render_node(scene, data.node_id, &data.object_to_world);
        }
    }

    pub fn render_post_processing(&self) {
        graphics::framebuffer::bind_default_framebuffer();
        let gpu_pp_config = GPUPostProcessConfig::from_cpu_config(&self.post_process_config);

        self.post_process_config_buffer.upload_data(&[gpu_pp_config]);

        graphics::utility::try_set_bindless_texture(
            &self.post_process_fs_quad.shader,
            "shadow_map",
            self.render_targets
                .shadow_framebuffer
                .depth_attachment()
                .unwrap()
                .bindless_handle(),
        );

        self.post_process_fs_quad.render();
    }

    fn render_mesh(&self, mesh: &Mesh, shader: &Shader, mat_props: &MaterialProperties) {
        graphics::utility::apply_mat_props(mat_props);
        shader.bind();
        mesh.vao().bind();

        unsafe {
            gl::DrawElements(
                gl::TRIANGLES,
                mesh.index_count(),
                mesh.index_format().to_gl_format(),
                std::ptr::null(),
            );
        }
    }

    fn render_node(&self, scene: &Scene, child_id: data::NodeID, object_to_world: &Mat4) {
        let child = scene.get_node(child_id);

        let child_mesh = scene.get_mesh(child.mesh_id);

        let child_material = scene.get_material(child.material_id);

        self.render_scene_mesh(scene, child_mesh, child_material, object_to_world);
    }

    fn render_scene_mesh(&self, scene: &Scene, mesh: &Mesh, material: &data::Material, model_matrix: &glam::Mat4) {
        mesh.vao().bind();

        let shader = scene.get_shader(material.shader_id);

        for texture_id in material.texture_ids.iter() {
            let texture = scene.get_texture(*texture_id);
            let texture_name = scene.get_texture_name(*texture_id);

            shader.map_bindless_texture(
                shader.find_uniform_location(texture_name).unwrap(),
                texture.bindless_handle(),
            );
        }

        graphics::utility::try_set_bindless_texture(
            shader,
            super::textures::CUBEMAP_TEXTURE,
            scene.cubemap().bindless_handle(),
        );

        graphics::utility::try_set_bindless_texture(
            shader,
            super::textures::AO_TEXTURE,
            self.ambient_occlusion.ao_compute_result_texture().bindless_handle(),
        );

        graphics::utility::try_set_bindless_texture(
            shader,
            super::textures::SHADOW_MAP,
            self.render_targets
                .shadow_framebuffer
                .depth_attachment()
                .unwrap()
                .bindless_handle(),
        );

        Renderer::upload_model_matrix(shader, model_matrix);

        if let Some(location) = shader.find_uniform_location("u_light_vp_matrix") {
            shader.set_uniform_mat4(location, &self.shadow_caster_ls_matrix.unwrap().to_cols_array());
        }

        self.render_mesh(mesh, shader, &material.material_properties);
    }

    fn upload_model_matrix(shader: &Shader, model_matrix: &glam::Mat4) {
        if let Some(location) = shader.find_uniform_location(SHADER_MODEL_MATRIX_UNIFORM_NAME) {
            shader.set_uniform_mat4(location, &model_matrix.to_cols_array());
        }
    }

    pub fn ssao_mut(&mut self) -> &mut AmbientOcclusion {
        &mut self.ambient_occlusion
    }

    pub fn ssao(&self) -> &AmbientOcclusion {
        &self.ambient_occlusion
    }
}

impl RenderTargets {
    pub const COLOR_TEXTURE_ATTACHMENT_INDEX: usize = 0;
    pub const NORMAL_TEXTURE_ATTACHMENT_INDEX: usize = 1;

    pub fn depth_texture(&self) -> &Texture2D {
        self.render_framebuffer.depth_attachment().unwrap()
    }

    pub fn color_texture(&self) -> &Texture2D {
        self.render_framebuffer
            .color_attachment(RenderTargets::COLOR_TEXTURE_ATTACHMENT_INDEX)
            .unwrap()
    }

    pub fn normal_texture(&self) -> &Texture2D {
        self.render_framebuffer
            .color_attachment(RenderTargets::NORMAL_TEXTURE_ATTACHMENT_INDEX)
            .unwrap()
    }

    pub fn new(resolution: (u32, u32)) -> Self {
        let mut render_framebuffer = Framebuffer::new(resolution);

        render_framebuffer.create_depth_attachment(texture::StorageFormat::Depth24);
        render_framebuffer.create_color_attachment(
            RenderTargets::COLOR_TEXTURE_ATTACHMENT_INDEX,
            texture::StorageFormat::SRGBA,
            texture::FilteringMode::Nearest,
        );
        render_framebuffer.create_color_attachment(
            RenderTargets::NORMAL_TEXTURE_ATTACHMENT_INDEX,
            texture::StorageFormat::RG16F,
            texture::FilteringMode::Nearest,
        );

        let mut shadow_framebuffer = Framebuffer::new(resolution);

        shadow_framebuffer.create_depth_attachment(texture::StorageFormat::Depth32);

        Self {
            render_framebuffer,
            shadow_framebuffer,
        }
    }
}
