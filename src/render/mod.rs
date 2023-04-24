pub mod context;
mod general;
pub mod gl;
mod shader;
mod vertex;

use crate::render::general::GeneralProgram;
use crate::render::gl::types::{GLsizei, GLsizeiptr, GLuint};
use crate::render::vertex::Vertex;
use crate::tweaks::{Model, ModelLoader};
use crate::BlueprintEntry;
pub use context::setup_opengl;
pub use gl::Gl;
use glutin::config::GlConfig;
use image::imageops::{flip_vertical_in_place, resize, FilterType};
use image::RgbImage;
use nalgebra_glm::{look_at, perspective, rotate_y, translation, Vec3};
use num_traits::FloatConst;
use std::mem::size_of;

const FORCED_SAMPLE_MULTIPLIER: u32 = 2;

pub fn perform_render(
    entries: &[BlueprintEntry],
    model_loader: &mut ModelLoader,
) -> Result<(), Box<dyn std::error::Error>> {
    // Render in 4k then scale it down by 4x to get better resolution
    let width = 1980 * FORCED_SAMPLE_MULTIPLIER;
    let height = 1080 * FORCED_SAMPLE_MULTIPLIER;

    println!("Creating graphics context");
    let (_, graphics) = context::setup_opengl(width, height);
    check_for_errors(&graphics.gl);
    println!("Created graphics context");

    println!(
        "Color buffer type: {:?}",
        graphics.gl_config.color_buffer_type()
    );
    // println!("Color buffer type: {:?}", graphics.gl_surface.);

    // let mut fbo = 0;
    // let mut rbo = 0;
    // let mut texture = 0;
    unsafe {
        // graphics.gl.GenFramebuffers(1, &mut fbo as *mut _);
        // graphics.gl.BindFramebuffer(gl::FRAMEBUFFER, fbo);
        // let mut framebuffer = 0;
        // let mut renderbuffer = 0;
        // graphics.gl.GenFramebuffers(1, &mut fbo);
        // graphics.gl.GenRenderbuffers(1, &mut rbo);
        println!("Created FBO and RBO");
        //
        // if graphics.gl.CheckFramebufferStatus(fbo) != gl::FRAMEBUFFER_COMPLETE {
        //     panic!("Failed to setup framebuffer!");
        // }
        //
        // graphics.gl.BindFramebuffer(gl::FRAMEBUFFER, fbo);
        // graphics.gl.BindRenderbuffer(gl::RENDERBUFFER, rbo);
        // graphics.gl.RenderbufferStorage(gl::RENDERBUFFER, gl::RGBA, width as GLsizei, height as GLsizei);
        // graphics.gl.FramebufferRenderbuffer(
        //     gl::FRAMEBUFFER,
        //     gl::COLOR_ATTACHMENT0,
        //     gl::RENDERBUFFER,
        //     rbo,
        // );
        check_for_errors(&graphics.gl);

        // graphics.gl.GenTextures(1, &mut texture);
        // graphics.gl.BindTexture(gl::TEXTURE_2D, texture);
        //
        // graphics.gl.TexImage2D(gl::TEXTURE_2D, 0, gl::RGB as GLint, width as GLsizei, height as GLsizei, 0, gl::RGB, gl::UNSIGNED_BYTE, 0 as *const c_void);
        //
        // graphics.gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
        // graphics.gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
        //
        // graphics.gl.FramebufferTexture(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, texture, 0);
        //
        // let mut draw_buffers = gl::COLOR_ATTACHMENT0;
        // graphics.gl.DrawBuffers(1, &mut draw_buffers);

        if graphics.gl.CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
            panic!("Failed to setup framebuffer!");
        }

        // graphics.gl.BindFramebuffer(gl::FRAMEBUFFER, fbo);
        graphics
            .gl
            .Viewport(0, 0, width as GLsizei, height as GLsizei);
        check_for_errors(&graphics.gl);

        println!("Assigned size to FBO");
    }

    let program = unsafe { GeneralProgram::build(&graphics.gl).unwrap() };
    println!("Built shader program");

    println!("Got shader uniforms");

    let mut vao = 0;

    unsafe {
        check_for_errors(&graphics.gl);
        // graphics.gl.Enable(gl::DEPTH_TEST);
        graphics.gl.UseProgram(program.program);
        check_for_errors(&graphics.gl);
        // graphics.gl.BindVertexArray(program.vao);
        // check_for_errors(&graphics.gl);

        graphics.gl.GenVertexArrays(1, &mut vao);
        check_for_errors(&graphics.gl);

        graphics.gl.BindVertexArray(vao);
        check_for_errors(&graphics.gl);

        graphics.gl.Enable(gl::DEPTH_TEST);
        check_for_errors(&graphics.gl);

        graphics.gl.ClearColor(0.1, 0.1, 0.1, 1.0);
        check_for_errors(&graphics.gl);
        graphics
            .gl
            .Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        check_for_errors(&graphics.gl);

        graphics.gl.DepthFunc(gl::LESS);
        check_for_errors(&graphics.gl);
    }
    println!("Enabled program and cleared buffer");

    let projection = perspective(width as f32 / height as f32, 1.2, 0.1, 100.0);
    let camera_pos = Vec3::new(-10.0, 10.0, -10.0);
    let light_direction = Vec3::new(0.0, -2.0, 1.0).normalize();

    let view = look_at(
        &camera_pos,
        &Vec3::new(0.0, 0.0, 0.0),
        &Vec3::new(0.0, 1.0, 0.0),
    );

    let mut vertex_list = Vec::new();
    for entry in entries {
        let model = match model_loader.load_model(&entry.internal_name) {
            Model::Missing => continue,
            Model::Resolved { model } => model,
        };

        let pos = translation(&Vec3::new(entry.x as f32, entry.l as f32, entry.y as f32));
        let pos = nalgebra_glm::scale(&pos, &Vec3::new(1.0, 1.0, -1.0));
        let building = rotate_y(&pos, entry.r as f32 * f32::PI() / 2.0);

        unsafe {
            program.uniforms.set_model(&graphics.gl, &building);
            program.uniforms.set_view(&graphics.gl, &view);
            program.uniforms.set_projection(&graphics.gl, &projection);

            program.uniforms.set_camera(&graphics.gl, &camera_pos);
            program
                .uniforms
                .set_light_direction(&graphics.gl, &light_direction);
            check_for_errors(&graphics.gl);
        }

        for group in model.data.objects.iter().flat_map(|x| x.groups.iter()) {
            vertex_list.clear();
            vertex::vertex_buffer_for_group(&mut vertex_list, model, group);

            unsafe {
                graphics.gl.BindVertexArray(vao);

                let vbo = load_vbo(&graphics.gl, &vertex_list);

                graphics.gl.BindBuffer(gl::ARRAY_BUFFER, vbo);

                Vertex::configure_vao(&graphics.gl);

                graphics
                    .gl
                    .DrawArrays(gl::TRIANGLES, 0, vertex_list.len() as GLsizei);

                graphics.gl.DeleteBuffers(1, &vbo as *const _);
                check_for_errors(&graphics.gl);
            }
        }
    }

    println!("Rendered all buildings");

    let mut buffer = vec![0u8; (width * height * 3) as usize];

    unsafe {
        graphics.gl.Finish();
        println!("Finished Rendering");
        check_for_errors(&graphics.gl);

        println!("Reading pixels to buffer");
        graphics.gl.ReadPixels(
            0,
            0,
            width as GLsizei,
            height as GLsizei,
            gl::RGB,
            gl::UNSIGNED_BYTE,
            buffer.as_mut_ptr() as *mut _,
        );
        check_for_errors(&graphics.gl);

        // TODO: Actually free resources... or not. It won't matter if the process gets cleaned up
        // by the OS after each run
        // graphics.gl.BindFramebuffer(gl::DRAW_FRAMEBUFFER, 0);
        // graphics.gl.BindRenderbuffer(gl::RENDERBUFFER, 0);
        // graphics.gl.DeleteFramebuffers(1, &fbo);
        // graphics.gl.DeleteRenderbuffers(1, &rbo);
    }

    println!("Freed rendering resources");

    let mut img = RgbImage::from_raw(width, height, buffer).unwrap();

    println!("Flipping image in memory");
    flip_vertical_in_place(&mut img);

    println!(
        "Shrinking image by a factor of {}",
        FORCED_SAMPLE_MULTIPLIER
    );
    let out = resize(
        &img,
        width / FORCED_SAMPLE_MULTIPLIER,
        height / FORCED_SAMPLE_MULTIPLIER,
        FilterType::Triangle,
    );

    println!("Saving image to disk");
    out.save("out_glutin.png").unwrap();

    println!("Wrote image!");

    Ok(())
}

pub fn load_vbo<T>(gl: &Gl, buffer: &[T]) -> GLuint {
    unsafe {
        let mut vbo = 0;
        gl.GenBuffers(1, &mut vbo);
        gl.BindBuffer(gl::ARRAY_BUFFER, vbo);

        gl.BufferData(
            gl::ARRAY_BUFFER,
            (size_of::<T>() * buffer.len()) as GLsizeiptr,
            buffer.as_ptr() as *const _,
            gl::STATIC_DRAW,
        );

        vbo
    }
}

#[track_caller]
pub fn check_for_errors(gl: &Gl) {
    let mut has_errored = false;
    loop {
        let err = unsafe { gl.GetError() };
        has_errored |= err != gl::NO_ERROR;

        match err {
            gl::NO_ERROR => {
                if has_errored {
                    panic!("Exited due to previous error")
                }
                return;
            }
            gl::INVALID_ENUM => println!("Invalid enum"),
            gl::INVALID_VALUE => println!("Invalid value"),
            gl::INVALID_OPERATION => println!("Invalid operation"),
            gl::STACK_OVERFLOW => println!("Stack overflow"),
            gl::STACK_UNDERFLOW => println!("Stack underflow"),
            gl::OUT_OF_MEMORY => println!("Out of memory"),
            gl::INVALID_FRAMEBUFFER_OPERATION => println!("Invalid framebuffer operation"),
            gl::CONTEXT_LOST => println!("Context lost"),
            x => println!("Unknown error code: 0x{:X}", x),
        }
    }
}
