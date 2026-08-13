use crate::{
    gl::{self},
    graphics::{
        self,
        texture::{FilteringMode, StorageFormat, Texture, Texture2D},
    },
};

pub struct Framebuffer {
    id: u32,
    res_x: u32,
    res_y: u32,

    depth_attachment: Option<Texture2D>,

    color_attachments: [Option<Texture2D>; Framebuffer::MAX_NUM_OF_COLOR_ATTACHMENTS],
}

impl Framebuffer {
    pub const MAX_NUM_OF_COLOR_ATTACHMENTS: usize = 10;

    pub fn set_active_render_target(&self, render_target_index: usize) {
        assert!(self.is_complete());

        assert!(self.color_attachments[render_target_index].is_some());

        unsafe {
            gl::NamedFramebufferDrawBuffer(self.id, gl::COLOR_ATTACHMENT0 + render_target_index as u32);
        }
    }

    pub fn depth_attachment(&self) -> Option<&Texture2D> {
        self.depth_attachment.as_ref()
    }

    pub fn color_attachment(&self, attachment_index: usize) -> Option<&Texture2D> {
        assert!(attachment_index < Framebuffer::MAX_NUM_OF_COLOR_ATTACHMENTS);
        self.color_attachments[attachment_index].as_ref()
    }

    pub fn clear_depth_attachment(&self) {
        self.depth_attachment
            .as_ref()
            .inspect(|texture| unsafe {
                gl::DepthMask(gl::TRUE);

                if texture.storage_format().is_depth_format() {
                    let depth = 1f32;
                    gl::ClearNamedFramebufferfv(self.id, gl::DEPTH, 0, &depth as *const f32);
                } else {
                    gl::ClearNamedFramebufferfi(self.id, gl::DEPTH_STENCIL, 0, 1.0f32, 0);
                }
            })
            .expect("Create depth attachment before calling clear");
    }

    pub fn create_depth_attachment(&mut self, storage_format: StorageFormat) -> &Texture2D {
        assert!(storage_format.is_depth_format());

        let attachment_texture = graphics::texture::Texture2D::create_texture(
            self.res_x as i32,
            self.res_y as i32,
            storage_format,
            graphics::texture::FilteringMode::Nearest,
            graphics::texture::WrappingMode::Clamp,
            false,
        );

        unsafe {
            gl::NamedFramebufferTexture(self.id, gl::DEPTH_ATTACHMENT, attachment_texture.id(), 0);
        }

        self.depth_attachment = Some(attachment_texture);

        self.depth_attachment.as_ref().unwrap()
    }

    pub fn create_color_attachment(
        &mut self,
        attachment_index: usize,
        storage_format: StorageFormat,
        filtering: FilteringMode,
    ) -> &Texture2D {
        let attachment = graphics::texture::Texture2D::create_texture(
            self.res_x as i32,
            self.res_y as i32,
            storage_format,
            filtering,
            graphics::texture::WrappingMode::Clamp,
            false,
        );

        unsafe {
            gl::NamedFramebufferTexture(
                self.id,
                gl::COLOR_ATTACHMENT0 + attachment_index as u32,
                attachment.id(),
                0,
            );
        }

        self.color_attachments[attachment_index] = Some(attachment);

        self.color_attachments[attachment_index].as_ref().unwrap()
    }

    pub fn new(resolution: (u32, u32)) -> Self {
        let mut id = 0;

        unsafe {
            gl::CreateFramebuffers(1, &mut id);
        }

        Self {
            id,
            res_x: resolution.0,
            res_y: resolution.1,
            depth_attachment: None,
            color_attachments: [const { None }; Framebuffer::MAX_NUM_OF_COLOR_ATTACHMENTS],
        }
    }

    pub fn is_complete(&self) -> bool {
        unsafe { gl::CheckNamedFramebufferStatus(self.id, gl::FRAMEBUFFER) == gl::FRAMEBUFFER_COMPLETE }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.id);
        }
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        if self.id != 0 {
            unsafe {
                gl::DeleteFramebuffers(1, &self.id);
            }
        }
    }
}

pub fn bind_default_framebuffer() {
    unsafe {
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
    }
}
