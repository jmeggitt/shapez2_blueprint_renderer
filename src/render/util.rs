use crate::render::gl::types::{GLenum, GLsizeiptr, GLuint};
use crate::render::{gl, Gl};
use std::ffi::c_void;
use std::mem::size_of;

pub unsafe fn load_vbo<T>(gl: &Gl, buffer: &[T]) -> GLuint {
    let mut vbo = 0;
    gl.GenBuffers(1, &mut vbo);
    gl.BindBuffer(gl::ARRAY_BUFFER, vbo);

    gl.BufferData(
        gl::ARRAY_BUFFER,
        (size_of::<T>() * buffer.len()) as GLsizeiptr,
        buffer.as_ptr() as *const c_void,
        gl::STATIC_DRAW,
    );

    vbo
}

#[track_caller]
pub fn check_for_errors(gl: &Gl) {
    // Don't allocate memory on the heap unless we find an error (equivalent to Vec::new)
    let mut error_queue = Vec::with_capacity(0);

    while let Some(error) = unsafe { get_gl_error(gl) } {
        error_queue.push(error);
    }

    if !error_queue.is_empty() {
        panic!(
            "OpenGL error queue contained contained errors: {:?}",
            error_queue
        )
    }
}

#[derive(Copy, Clone, Debug)]
enum GlError {
    InvalidEnum,
    InvalidValue,
    InvalidOperation,
    StackOverflow,
    StackUnderflow,
    OutOfMemory,
    InvalidFramebufferOperation,
    ContextLost,
    UnknownErrorCode(GLenum),
}

#[inline]
unsafe fn get_gl_error(gl: &Gl) -> Option<GlError> {
    match gl.GetError() {
        gl::NO_ERROR => None,
        gl::INVALID_ENUM => Some(GlError::InvalidEnum),
        gl::INVALID_VALUE => Some(GlError::InvalidValue),
        gl::INVALID_OPERATION => Some(GlError::InvalidOperation),
        gl::STACK_OVERFLOW => Some(GlError::StackOverflow),
        gl::STACK_UNDERFLOW => Some(GlError::StackUnderflow),
        gl::OUT_OF_MEMORY => Some(GlError::OutOfMemory),
        gl::INVALID_FRAMEBUFFER_OPERATION => Some(GlError::InvalidFramebufferOperation),
        gl::CONTEXT_LOST => Some(GlError::ContextLost),
        x => Some(GlError::UnknownErrorCode(x)),
    }
}
