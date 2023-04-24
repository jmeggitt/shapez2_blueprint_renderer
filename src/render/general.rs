use crate::c_str;
use crate::render::gl::types::{GLint, GLuint};
use crate::render::shader::{build_program, ShaderError};
use crate::render::{check_for_errors, gl, Gl};
use nalgebra_glm::{Mat4, Vec3};

pub struct GeneralProgram {
    pub program: GLuint,
    pub uniforms: GeneralProgramUniforms,
}

impl GeneralProgram {
    pub unsafe fn build(gl: &Gl) -> Result<Self, ShaderError> {
        let vert = c_str!(include_str!("../vert.glsl"));
        let frag = c_str!(include_str!("../frag.glsl"));

        let program = build_program(gl, vert, frag)?;
        gl.UseProgram(program);

        Ok(GeneralProgram {
            program,
            uniforms: GeneralProgramUniforms::from_program(gl, program),
        })
    }
}

pub struct GeneralProgramUniforms {
    model: GLint,
    view: GLint,
    projection: GLint,
    camera: GLint,
    light_direction: GLint,
}

impl GeneralProgramUniforms {
    unsafe fn from_program(gl: &Gl, program: GLuint) -> Self {
        let uniforms = GeneralProgramUniforms {
            model: gl.GetUniformLocation(program, c_str!("model").as_ptr()),
            view: gl.GetUniformLocation(program, c_str!("view").as_ptr()),
            projection: gl.GetUniformLocation(program, c_str!("projection").as_ptr()),
            camera: gl.GetUniformLocation(program, c_str!("camera").as_ptr()),
            light_direction: gl.GetUniformLocation(program, c_str!("lightDirection").as_ptr()),
        };

        check_for_errors(gl);
        uniforms
    }

    pub unsafe fn set_model(&self, gl: &Gl, model: &Mat4) {
        gl.UniformMatrix4fv(self.model, 1, gl::FALSE, model.as_ptr() as *const _);
    }

    pub unsafe fn set_view(&self, gl: &Gl, view: &Mat4) {
        gl.UniformMatrix4fv(self.view, 1, gl::FALSE, view.as_ptr() as *const _);
    }

    pub unsafe fn set_projection(&self, gl: &Gl, projection: &Mat4) {
        gl.UniformMatrix4fv(
            self.projection,
            1,
            gl::FALSE,
            projection.as_ptr() as *const _,
        );
    }

    pub unsafe fn set_camera(&self, gl: &Gl, x: &Vec3) {
        gl.Uniform3fv(self.camera, 1, x.as_ptr() as *const _);
    }

    pub unsafe fn set_light_direction(&self, gl: &Gl, x: &Vec3) {
        gl.Uniform3fv(self.light_direction, 1, x.as_ptr() as *const _);
    }
}
