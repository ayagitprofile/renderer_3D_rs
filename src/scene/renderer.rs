use glam::{Mat4, Vec3, Vec4};

use super::{data, scene::Scene};
use crate::ambient_occlusion::AmbientOcclusion;
use crate::camera::Camera;
use crate::graphics::framebuffer::Framebuffer;
use crate::graphics::material_properties::MaterialProperties;
use crate::graphics::mesh::Mesh;
use crate::graphics::shader::Shader;
use crate::graphics::texture::{self, Texture};
use crate::scene::skybox::Skybox;
use crate::shader_source::ShaderSource;
use crate::{ambient_occlusion, gl, graphics};

const SHADER_MODEL_MATRIX_UNIFORM_NAME: &str = "u_model_matrix";

pub struct Renderer {
    current_mat_props: MaterialProperties,
    draw_call_data: Vec<DrawCallData>,
    depth_prepass_shader: (Shader, MaterialProperties),
    skybox: Skybox,

    render_targets: RenderTargets,

    ambient_occlusion: ambient_occlusion::AmbientOcclusion,

    post_process_fs_quad: graphics::fullscreen_quad::FullscreenQuad,
}

pub struct RenderTargets {
    framebuffer: Framebuffer,

    color_texture: texture::Texture2D,
    depth_texture: texture::Texture2D,
    normal_texture: texture::Texture2D,
}

struct DrawCallData {
    object_to_world: Mat4,
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
    fn new(node_id: data::NodeID, object_to_world: Mat4) -> Self {
        Self {
            node_id,
            object_to_world,
        }
    }
}

impl Renderer {
    pub fn prepare_rendering_data(&mut self, scene: &Scene, camera: &Camera) -> RenderingStats {
        self.draw_call_data.clear();

        let camera_frustum =
            CameraFrustum::from_view_projection_matrix(camera.projection_matrix() * camera.view_matrix());

        let mut total_objects = 0;

        for root_node_id in scene.root_node_iter() {
            total_objects += 1;

            let root_node = scene.get_node(*root_node_id);

            let object_to_world = *root_node.transform.model_matrix();

            let position = object_to_world.w_axis.truncate();

            let mut aabb = AABBVec3::from_scene_aabb(&root_node.bounding_box);
            aabb.max += position;
            aabb.min += position;

            if camera_frustum.intersects_aabb(&aabb) {
                self.draw_call_data
                    .push(DrawCallData::new(*root_node_id, object_to_world));
            }

            for child_id in root_node.children_iter() {
                total_objects += 1;

                let child_node = scene.get_node(*child_id);
                let child_object_to_world = root_node.transform.model_matrix() * child_node.transform.model_matrix();

                let child_position = child_object_to_world.w_axis.truncate();
                let mut child_aabb = AABBVec3::from_scene_aabb(&child_node.bounding_box);
                child_aabb.max += child_position;
                child_aabb.min += child_position;

                if camera_frustum.intersects_aabb(&child_aabb) {
                    self.draw_call_data
                        .push(DrawCallData::new(*child_id, child_object_to_world));
                }
            }
        }

        self.draw_call_data.sort_by(|a, b| {
            let a_position = a.object_to_world.w_axis.truncate();
            let a_distance_to_camera = camera.transform.position().distance_squared(a_position);

            let b_position = b.object_to_world.w_axis.truncate();
            let b_distance_to_camera = camera.transform.position().distance_squared(b_position);

            a_distance_to_camera.total_cmp(&b_distance_to_camera)
        });

        RenderingStats {
            total_objects: total_objects,
            visible_objects: self.draw_call_data.len() as u32,
        }
    }

    pub fn new(resolution: (u32, u32)) -> Self {
        let depth_prepass_source =
            ShaderSource::load_from_file(std::path::Path::new("assets/shaders/depth_prepass_shader.glsl"));

        let render_targets = RenderTargets::new(resolution.0, resolution.1);

        let ao = ambient_occlusion::AmbientOcclusion::new(resolution.0, resolution.1);
        ao.set_input_textures(&render_targets.depth_texture, &render_targets.normal_texture);

        let fs_quad = graphics::fullscreen_quad::FullscreenQuad::new(std::path::Path::new(
            "assets/shaders/post_process_shader.glsl",
        ));

        graphics::utility::try_set_bindless_texture(
            &fs_quad.shader,
            super::textures::FRAMEBUFFER_COLOR_TEXTURE,
            render_targets.color_texture.bindless_handle(),
        );

        let renderer = Renderer {
            current_mat_props: MaterialProperties::DEFAULT,
            draw_call_data: Vec::with_capacity(128),
            depth_prepass_shader: (depth_prepass_source.compile(), *depth_prepass_source.mat_props()),
            skybox: Skybox::new(),
            render_targets: render_targets,
            ambient_occlusion: ao,
            post_process_fs_quad: fs_quad,
        };

        graphics::utility::try_set_bindless_texture(
            &renderer.skybox.shader,
            "cubemap_texture",
            renderer.skybox.cubemap.bindless_handle(),
        );

        renderer
    }

    pub fn render_depth_prepass(&self, scene: &Scene) {
        self.render_targets
            .framebuffer
            .set_active_render_target(RenderTargets::NORMAL_TEXTURE_ATTACHMENT_INDEX);

        let shader = &self.depth_prepass_shader.0;
        shader.bind();

        graphics::utility::apply_mat_props(&self.depth_prepass_shader.1);

        let model_matrix_uniform_location = shader.find_uniform_location("u_model_matrix").unwrap();

        for data in self.draw_call_data.iter() {
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
        self.render_targets
            .framebuffer
            .set_active_render_target(RenderTargets::COLOR_TEXTURE_ATTACHMENT_INDEX);

        for data in self.draw_call_data.iter() {
            self.render_node(scene, data.node_id, &data.object_to_world);
        }

        self.render_mesh(&self.skybox.mesh, &self.skybox.shader, &self.skybox.mat_props);
    }

    pub fn render_post_processing(&self) {
        graphics::framebuffer::bind_default_framebuffer();
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
            self.skybox.cubemap.bindless_handle(),
        );

        graphics::utility::try_set_bindless_texture(
            shader,
            super::textures::AO_TEXTURE,
            self.ambient_occlusion.ao_compute_result_texture().bindless_handle(),
        );

        Renderer::upload_model_matrix(shader, model_matrix);

        self.render_mesh(mesh, shader, &material.material_properties);
    }

    fn upload_model_matrix(shader: &Shader, model_matrix: &glam::Mat4) {
        if let Some(location) = shader.find_uniform_location(SHADER_MODEL_MATRIX_UNIFORM_NAME) {
            shader.set_uniform_mat4(location, &model_matrix.to_cols_array());
        }
    }

    pub fn new_frame(&self) {
        self.render_targets.framebuffer.clear_all_attachments();
        self.render_targets.framebuffer.bind();
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

    fn create_texture_set(width: u32, height: u32) -> (texture::Texture2D, texture::Texture2D, texture::Texture2D) {
        let (width, height) = (width as i32, height as i32);

        let color_texture = texture::Texture2D::create_texture(
            width,
            height,
            texture::StorageFormat::SRGBA,
            texture::FilteringMode::Bilinear,
            texture::WrappingMode::Clamp,
            false,
        );

        let depth_texture = texture::Texture2D::create_texture(
            width,
            height,
            texture::StorageFormat::Depth24FStencil,
            texture::FilteringMode::Nearest,
            texture::WrappingMode::Clamp,
            false,
        );

        let normal_texture = texture::Texture2D::create_texture(
            width,
            height,
            texture::StorageFormat::RG16F,
            texture::FilteringMode::Nearest,
            texture::WrappingMode::Clamp,
            false,
        );

        (color_texture, depth_texture, normal_texture)
    }

    fn attach_textures(&mut self, color: texture::Texture2D, depth: texture::Texture2D, normal: texture::Texture2D) {
        self.color_texture = color;
        self.depth_texture = depth;
        self.normal_texture = normal;

        self.framebuffer
            .set_depth_texture_render_target(self.depth_texture.id(), self.depth_texture.storage_format());
        self.framebuffer
            .set_color_texture_render_target(self.color_texture.id(), RenderTargets::COLOR_TEXTURE_ATTACHMENT_INDEX);
        self.framebuffer
            .set_color_texture_render_target(self.normal_texture.id(), RenderTargets::NORMAL_TEXTURE_ATTACHMENT_INDEX);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let (color_texture, depth_texture, normal_texture) = RenderTargets::create_texture_set(width, height);
        self.attach_textures(color_texture, depth_texture, normal_texture);
    }

    pub fn new(width: u32, height: u32) -> Self {
        let (color_texture, depth_texture, normal_texture) = RenderTargets::create_texture_set(width, height);

        let mut framebuffer = Framebuffer::new();

        framebuffer.set_depth_texture_render_target(depth_texture.id(), depth_texture.storage_format());
        framebuffer.set_color_texture_render_target(color_texture.id(), RenderTargets::COLOR_TEXTURE_ATTACHMENT_INDEX);
        framebuffer
            .set_color_texture_render_target(normal_texture.id(), RenderTargets::NORMAL_TEXTURE_ATTACHMENT_INDEX);

        Self {
            framebuffer,
            color_texture,
            depth_texture,
            normal_texture,
        }
    }
}
