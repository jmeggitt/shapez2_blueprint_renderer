use std::error::Error;
use std::ffi::CStr;
use std::fmt::{Display, Formatter};
use crate::render::{check_for_errors, Gl, gl};
use crate::render::gl::types::{GLchar, GLint, GLsizei, GLuint};


#[inline]
pub fn build_program(gl: &Gl, vert: &CStr, frag: &CStr) -> Result<GLuint, ShaderError> {
    build_with_geometry(gl, vert, frag, None)
}

pub fn build_with_geometry(gl: &Gl, vert: &CStr, frag: &CStr, geom: Option<&CStr>) -> Result<GLuint, ShaderError> {
    check_for_errors(&gl);
    let vert_handle = compile_shader(gl, gl::VERTEX_SHADER, vert)?;
    check_for_errors(&gl);

    let frag_handle = compile_shader(gl, gl::FRAGMENT_SHADER, frag)?;
    check_for_errors(&gl);

    let geom_handle = match geom {
        Some(src) => Some(compile_shader(gl, gl::GEOMETRY_SHADER, src)?),
        None => None,
    };
    check_for_errors(&gl);

    unsafe {
        let program = gl.CreateProgram();
        check_for_errors(&gl);

        // Attach shaders to program
        gl.AttachShader(program, vert_handle);
        gl.AttachShader(program, frag_handle);
        if let Some(geom_handle) = &geom_handle {
            gl.AttachShader(program, *geom_handle);
        }
        check_for_errors(&gl);

        // Link program and check for errors
        gl.LinkProgram(program);
        gl.UseProgram(program);
        // let errors = check_for_shader_errors(gl, program);
        let errors = check_for_program_errors(gl, program);
        check_for_errors(&gl);

        // Cleanup shaders
        gl.DeleteShader(vert_handle);
        gl.DeleteShader(frag_handle);
        if let Some(geom_handle) = geom_handle {
            gl.DeleteShader(geom_handle);
        }
        check_for_errors(&gl);

        // If there were no errors then return the program
        errors.map(|_| program)
    }
}

fn check_for_program_errors(gl: &Gl, program: GLuint) -> Result<(), ShaderError> {
    unsafe {
        let mut code: GLint = gl::TRUE as GLint;
        gl.GetProgramiv(program, gl::LINK_STATUS, &mut code as *mut _);
        check_for_errors(&gl);

        if code == gl::TRUE as GLint {
            return Ok(());
        }

        // Create buffer and read shader error message
        let mut buffer = vec![0u8; 4096];
        let mut length: GLsizei = 0;

        gl.GetProgramInfoLog(program, buffer.len() as GLsizei, &mut length, buffer.as_mut_ptr() as *mut GLchar);
        check_for_errors(&gl);

        return Err(ShaderError {
            code,
            msg: String::from_utf8_lossy(&buffer[..length as usize]).into_owned(),
        });
    }
}

fn check_for_shader_errors(gl: &Gl, shader: GLuint) -> Result<(), ShaderError> {
    unsafe {
        let mut code: GLint = 0;
        gl.GetShaderiv(shader, gl::COMPILE_STATUS, &mut code as *mut _);
        check_for_errors(&gl);

        if code == gl::TRUE as GLint  {
            return Ok(());
        }

        // Create buffer and read shader error message
        let mut buffer = vec![0u8; 4096];
        let mut length: GLsizei = 0;

        gl.GetShaderInfoLog(shader, buffer.len() as GLsizei, &mut length, buffer.as_mut_ptr() as *mut GLchar);
        check_for_errors(&gl);

        return Err(ShaderError {
            code,
            msg: String::from_utf8_lossy(&buffer[..length as usize]).into_owned(),
        });
    }
}


#[derive(Debug)]
pub struct ShaderError {
    code: GLint,
    msg: String,
}

impl Display for ShaderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Shader error (code: {}):\n{}", self.code, &self.msg)
    }
}

impl Error for ShaderError {}

pub fn compile_shader(gl: &Gl, shader_type: GLuint, src: &CStr) -> Result<GLuint, ShaderError> {
    unsafe {
        let shader = gl.CreateShader(shader_type);
        check_for_errors(&gl);

        gl.ShaderSource(shader, 1, &[src.as_ptr()] as *const _, std::ptr::null());
        check_for_errors(&gl);

        gl.CompileShader(shader);
        check_for_errors(&gl);

        // check_for_program_errors(gl, shader).map(|_| shader)
        check_for_shader_errors(gl, shader).map(|_| shader)
    }
}

