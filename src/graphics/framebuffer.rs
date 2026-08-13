use crate::{
    gl::{self},
    graphics::texture::{StorageFormat, Texture, Texture2D},
};

#[derive(Clone, Copy)]
pub enum RenderTarget {
    Texture2D { id: u32 },
}

pub struct Framebuffer {
    id: u32,
    color_target_slots: [Option<RenderTarget>; Framebuffer::MAX_RENDER_TARGETS],
    depth_target: Option<(RenderTarget, u32)>,
}

const CLEAR_COLOR: [f32; 4] = [0f32; 4];

impl Framebuffer {
    pub const MAX_RENDER_TARGETS: usize = 6;

    pub fn set_depth_texture_render_target(&mut self, texture: &Texture2D) {
        assert!(
            texture.storage_format() == StorageFormat::Depth16F
                || texture.storage_format() == StorageFormat::Depth24F
                || texture.storage_format() == StorageFormat::Depth32F
        );

        self.depth_target = Some((RenderTarget::Texture2D { id: texture.id() }, gl::DEPTH));

        unsafe {
            gl::NamedFramebufferTexture(self.id, gl::DEPTH_ATTACHMENT, texture.id(), 0);
        }
    }

    pub fn set_color_texture_render_target(&mut self, texture_id: u32, attachment_slot_index: usize) {
        assert!(attachment_slot_index <= Framebuffer::MAX_RENDER_TARGETS);

        self.color_target_slots[attachment_slot_index] = Some(RenderTarget::Texture2D { id: texture_id });

        unsafe {
            gl::NamedFramebufferTexture(
                self.id,
                gl::COLOR_ATTACHMENT0 + attachment_slot_index as u32,
                texture_id,
                0,
            );
        }
    }

    pub fn disable_render_targets(&self) {
        unsafe {
            gl::NamedFramebufferDrawBuffer(self.id, 0);
        }
    }

    pub fn set_active_render_target(&self, render_target_index: usize) {
        assert!(self.is_complete());

        assert!(self.color_target_slots[render_target_index].is_some());

        unsafe {
            gl::NamedFramebufferDrawBuffer(self.id, gl::COLOR_ATTACHMENT0 + render_target_index as u32);
            // gl::ClearNamedFramebufferfv(self.id, gl::COLOR, render_target_index as i32, CLEAR_COLOR.as_ptr());
        }
    }

    pub fn set_active_render_targets(&self, render_target_indexes: &[usize]) {
        assert!(self.is_complete());

        let mut target_buffer = [0u32; Framebuffer::MAX_RENDER_TARGETS];
        let mut count = 0;

        for target_index in render_target_indexes {
            assert!(self.color_target_slots[*target_index].is_some());

            target_buffer[count] = gl::COLOR_ATTACHMENT0 + *target_index as u32;
            count += 1;
        }

        unsafe {
            gl::NamedFramebufferDrawBuffers(self.id, count as i32, target_buffer.as_ptr());
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

    pub fn clear_all_attachments(&self) {
        assert!(self.is_complete());
        const COLOR: [f32; 4] = [0f32; 4];

        unsafe {
            let clear_target = self.depth_target.unwrap().1;
            gl::DepthMask(gl::TRUE);
            if clear_target == gl::DEPTH {
                let depth = 1.0f32;
                gl::ClearNamedFramebufferfv(self.id, clear_target, 0, &depth as *const f32);
            } else {
                gl::ClearNamedFramebufferfi(self.id, clear_target, 0, 1.0f32, 0);
            }
        }

        let mut render_attachments = [0u32; Framebuffer::MAX_RENDER_TARGETS];
        let mut count = 0i32;

        for i in 0..self.color_target_slots.len() {
            if self.color_target_slots[i].is_some() {
                render_attachments[count as usize] = gl::COLOR_ATTACHMENT0 + i as u32;
                count += 1;
            }
        }

        unsafe {
            gl::NamedFramebufferDrawBuffers(self.id, count, render_attachments.as_ptr());

            for i in 0..count {
                gl::ClearNamedFramebufferfv(self.id, gl::COLOR, i, COLOR.as_ptr());
            }
        }
    }

    pub fn new() -> Self {
        let mut id = 0u32;

        unsafe {
            gl::CreateFramebuffers(1, &mut id as *mut u32);
        }

        Self {
            id: id,
            color_target_slots: [None; Framebuffer::MAX_RENDER_TARGETS],
            depth_target: None,
        }
    }
}

pub fn bind_default_framebuffer() {
    unsafe {
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteFramebuffers(1, &self.id as *const u32);
        }
    }
}

impl Framebuffer {}
