use glam::{FloatExt, Vec3, vec3, vec4};
use rand::Rng;

use crate::{
    gl,
    graphics::{
        self,
        buffer::GraphicsBuffer,
        shader::{self, ComputeShader},
        texture::{self, StorageFormat, Texture},
    },
    scene,
    shader_source::ShaderSource,
};

const COMPUTE_THREADS_PER_WORK_GROUP_X: u32 = 16;
const COMPUTE_THREADS_PER_WORK_GROUP_Y: u32 = 16;

const KERNEL_SAMPLE_COUNT: usize = 32;
const RANDOM_DIRECTION_TEXTURE_WIDTH: usize = 8;

const COMPUTE_SHADER_RANDOM_DIRECTION_TEXTURE_NAME: &str = "random_direction_texture";
const COMPUTE_SHADER_KERNEL_DATA_BUFFER_BINDING: u32 = scene::buffers::SHADER_FREE_DATA_BUFFER_BINDING_INDEX;

#[repr(C)]
#[derive(Clone, Copy)]
struct SSAOComputeUniformData {
    pub radius: f32,
    pub depth_bias: f32,
    pub strength: f32,
}

pub struct AmbientOcclusion {
    ao_compute_shader: ComputeShader,
    ao_blur_compute_shader: ComputeShader,

    ao_compute_stage_texture: texture::Texture2D,

    output_texture: texture::Texture2D,

    _random_directions_texture: texture::Texture2D,
    kernel_samples_buffer: GraphicsBuffer,

    uniform_data: SSAOComputeUniformData,
    uniform_data_buffer: GraphicsBuffer,

    enable_blur_pass: bool,
}

impl AmbientOcclusion {
    pub fn ao_compute_result_texture(&self) -> &texture::Texture2D {
        &self.output_texture
    }

    pub fn set_input_textures(&self, depth_texture: &texture::Texture2D, normal_texture: &texture::Texture2D) {
        graphics::utility::try_set_bindless_texture(
            self.ao_blur_compute_shader.underlying_shader(),
            scene::textures::FRAMEBUFFER_DEPTH_TEXTURE,
            depth_texture.bindless_handle(),
        );

        graphics::utility::try_set_bindless_texture(
            self.ao_compute_shader.underlying_shader(),
            scene::textures::FRAMEBUFFER_DEPTH_TEXTURE,
            depth_texture.bindless_handle(),
        );

        graphics::utility::try_set_bindless_texture(
            self.ao_compute_shader.underlying_shader(),
            scene::textures::FRAMEBUFFER_NORMAL_TEXTURE,
            normal_texture.bindless_handle(),
        );
    }

    pub fn compute_ambient_occlusion(&self) {
        if self.uniform_data.strength < 0.01 {
            self.output_texture.clear([1f32; 4]);
            return;
        }

        let uniform_data = SSAOComputeUniformData {
            radius: self.uniform_data.radius.clamp(0f32, 100f32),
            depth_bias: self.uniform_data.depth_bias.clamp(0f32, 1f32),
            strength: self.uniform_data.strength.clamp(0f32, 10f32),
        };

        self.kernel_samples_buffer.set_binding(
            graphics::buffer::BindingTarget::ShaderStorageBuffer,
            COMPUTE_SHADER_KERNEL_DATA_BUFFER_BINDING,
        );

        self.uniform_data_buffer
            .set_binding(graphics::buffer::BindingTarget::UniformBuffer, 0);

        self.uniform_data_buffer.upload_data(&[uniform_data]);

        let (texture_width, texture_height) = (self.output_texture.width() as u32, self.output_texture.height() as u32);

        let (work_groups_x, work_groups_y) = (
            (texture_width + COMPUTE_THREADS_PER_WORK_GROUP_X - 1) / COMPUTE_THREADS_PER_WORK_GROUP_X,
            (texture_height + COMPUTE_THREADS_PER_WORK_GROUP_Y - 1) / COMPUTE_THREADS_PER_WORK_GROUP_Y,
        );

        if self.enable_blur_pass == false {
            unsafe {
                gl::BindImageTexture(0, self.output_texture.id(), 0, gl::FALSE, 0, gl::WRITE_ONLY, gl::R16F);
            }

            self.ao_compute_shader.dispatch(
                work_groups_x,
                work_groups_y,
                Some(shader::ComputeMemoryBarrier::TextureFetch),
            );

            return;
        }

        // ao calculation pass
        {
            unsafe {
                gl::BindImageTexture(
                    0,
                    self.ao_compute_stage_texture.id(),
                    0,
                    gl::FALSE,
                    0,
                    gl::WRITE_ONLY,
                    gl::R16F,
                );
            }

            self.ao_compute_shader.dispatch(
                work_groups_x,
                work_groups_y,
                Some(shader::ComputeMemoryBarrier::ImageAccess),
            );
        }

        let direction_uniform_location = self
            .ao_blur_compute_shader
            .underlying_shader()
            .find_uniform_location("u_horizontal")
            .unwrap();

        // ao blur pass
        {
            unsafe {
                gl::BindImageTexture(
                    0,
                    self.ao_compute_stage_texture.id(),
                    0,
                    gl::FALSE,
                    0,
                    gl::READ_ONLY,
                    gl::R16F,
                );

                gl::BindImageTexture(1, self.output_texture.id(), 0, gl::FALSE, 0, gl::WRITE_ONLY, gl::R16F);
            }

            self.ao_blur_compute_shader
                .underlying_shader()
                .set_uniform_u32(direction_uniform_location, 1);

            self.ao_blur_compute_shader.dispatch(
                work_groups_x,
                work_groups_y,
                Some(shader::ComputeMemoryBarrier::ImageAccess),
            );

            self.ao_blur_compute_shader
                .underlying_shader()
                .set_uniform_u32(direction_uniform_location, 0);

            self.ao_blur_compute_shader.dispatch(
                work_groups_x,
                work_groups_y,
                Some(shader::ComputeMemoryBarrier::TextureFetch),
            );
        }
    }

    pub fn set_radius(&mut self, radius: f32) {
        self.uniform_data.radius = radius;
    }

    pub fn set_depth_bias(&mut self, depth_bias: f32) {
        self.uniform_data.depth_bias = depth_bias;
    }

    pub fn set_strength(&mut self, strength: f32) {
        self.uniform_data.strength = strength;
    }

    pub fn radius(&self) -> f32 {
        self.uniform_data.radius
    }

    pub fn depth_bias(&self) -> f32 {
        self.uniform_data.depth_bias
    }

    pub fn strength(&self) -> f32 {
        self.uniform_data.strength
    }

    pub fn enable_blur_pass(&mut self) -> &mut bool {
        &mut self.enable_blur_pass
    }

    pub fn new(texture_width: u32, texture_height: u32) -> Self {
        let mut ao_source = ShaderSource::load_from_file(std::path::Path::new("assets/shaders/ao_compute_shader.glsl"));

        ao_source.insert_line(
            crate::shader_source::ShaderParsingTarget::Compute,
            &format!("layout(local_size_x = {COMPUTE_THREADS_PER_WORK_GROUP_X}, local_size_y = {COMPUTE_THREADS_PER_WORK_GROUP_Y}) in;"),
        );

        let ao_compute_shader = ao_source.compile_compute();

        let ao_blur_shader =
            ShaderSource::load_from_file(std::path::Path::new("assets/shaders/ao_blur_shader.glsl")).compile_compute();

        const SAMPLE_COUNT: usize = RANDOM_DIRECTION_TEXTURE_WIDTH * RANDOM_DIRECTION_TEXTURE_WIDTH;

        let random_directions = AmbientOcclusion::create_random_directions::<SAMPLE_COUNT>();
        let kernel_samples =
            AmbientOcclusion::create_sample_kernel::<KERNEL_SAMPLE_COUNT>().map(|el| vec4(el[0], el[1], el[2], 0f32));

        let random_direction_texture = texture::Texture2D::create_texture(
            RANDOM_DIRECTION_TEXTURE_WIDTH as i32,
            RANDOM_DIRECTION_TEXTURE_WIDTH as i32,
            texture::StorageFormat::RGB16F,
            texture::FilteringMode::Nearest,
            texture::WrappingMode::Repeat,
            false,
        );

        random_direction_texture.upload_data_f32(
            RANDOM_DIRECTION_TEXTURE_WIDTH as u32,
            RANDOM_DIRECTION_TEXTURE_WIDTH as u32,
            3,
            random_directions.as_ptr() as *const f32,
        );

        ao_compute_shader.underlying_shader().map_bindless_texture(
            ao_compute_shader
                .underlying_shader()
                .find_uniform_location(COMPUTE_SHADER_RANDOM_DIRECTION_TEXTURE_NAME)
                .unwrap(),
            random_direction_texture.bindless_handle(),
        );

        let mut kernel_samples_buffer = graphics::buffer::GraphicsBuffer::new();
        kernel_samples_buffer.allocate(kernel_samples.as_slice(), graphics::buffer::Usage::Static);

        let uniform_data = SSAOComputeUniformData {
            radius: 0.2f32,
            depth_bias: 0.02f32,
            strength: 1f32,
        };

        let mut uniform_data_buffer = GraphicsBuffer::new();
        uniform_data_buffer.allocate(&[uniform_data], graphics::buffer::Usage::Dynamic);

        let (format, filtering, wrapping, generate_mip_maps) = (
            StorageFormat::R16F,
            texture::FilteringMode::Nearest,
            texture::WrappingMode::Clamp,
            false,
        );

        let ao_compute_stage_texture = texture::Texture2D::create_texture(
            texture_width as i32,
            texture_height as i32,
            format,
            filtering,
            wrapping,
            generate_mip_maps,
        );

        let output_texture = texture::Texture2D::create_texture(
            texture_width as i32,
            texture_height as i32,
            format,
            filtering,
            wrapping,
            generate_mip_maps,
        );

        Self {
            ao_compute_shader,
            ao_blur_compute_shader: ao_blur_shader,

            _random_directions_texture: random_direction_texture,
            kernel_samples_buffer,

            output_texture: output_texture,
            ao_compute_stage_texture: ao_compute_stage_texture,

            uniform_data,
            uniform_data_buffer,

            enable_blur_pass: true,
        }
    }

    fn create_random_directions<const SAMPLE_COUNT: usize>() -> [Vec3; SAMPLE_COUNT] {
        let mut rng = rand::rng();

        let mut directions = [Vec3::ZERO; SAMPLE_COUNT];

        for i in 0..(SAMPLE_COUNT) {
            let angle = rng.random_range(0f32..std::f32::consts::TAU);

            directions[i] = vec3(angle.cos(), angle.sin(), 0f32);
        }

        directions
    }

    fn create_sample_kernel<const SAMPLE_COUNT: usize>() -> [glam::Vec3; SAMPLE_COUNT] {
        let mut rng = rand::rng();

        let mut points = [glam::Vec3::ZERO; SAMPLE_COUNT];

        for i in 0..SAMPLE_COUNT {
            let mut direction;

            loop {
                let x = rng.random_range(-1.0f32..1.0);
                let y = rng.random_range(-1.0f32..1.0);
                let z = rng.random_range(0.0f32..1.0);

                direction = vec3(x, y, z);

                if direction.length_squared() > 0.0001 {
                    break;
                }
            }

            direction = direction.normalize();

            let t = i as f32 / (SAMPLE_COUNT - 1) as f32;

            let scale = 0.1f32.lerp(1.0, t * t);

            points[i] = direction * scale;
        }

        points
    }
}
