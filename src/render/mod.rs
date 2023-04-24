pub mod gl;
pub mod context;
mod vertex;
mod shader;
mod general;

use std::ffi::c_void;
use std::fs::File;
use std::io::Write;
use std::mem::size_of;
use glutin::config::GlConfig;
use image::imageops::{FilterType, flip_vertical_in_place, resize};
use image::RgbImage;
use nalgebra_glm::{look_at, perspective, rotate_y, translation, Vec3};
use num_traits::FloatConst;
use png::{BitDepth, ColorType, Encoder};
pub use gl::Gl;
pub use context::setup_opengl;
use crate::{BlueprintEntry, c_str};
use crate::render::general::GeneralProgram;
use crate::render::gl::types::{GLint, GLsizei, GLsizeiptr, GLuint};
use crate::render::vertex::Vertex;
use crate::tweaks::{Model, ModelLoader};

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


    println!("Color buffer type: {:?}", graphics.gl_config.color_buffer_type());
    // println!("Color buffer type: {:?}", graphics.gl_surface.);

    let mut fbo = 0;
    let mut rbo = 0;
    let mut texture = 0;
    unsafe {

        // graphics.gl.GenFramebuffers(1, &mut fbo as *mut _);
        // graphics.gl.BindFramebuffer(gl::FRAMEBUFFER, fbo);
        // let mut framebuffer = 0;
        // let mut renderbuffer = 0;
        graphics.gl.GenFramebuffers(1, &mut fbo);
        graphics.gl.GenRenderbuffers(1, &mut rbo);
        println!("Created FBO and RBO");
        //
        // if graphics.gl.CheckFramebufferStatus(fbo) != gl::FRAMEBUFFER_COMPLETE {
        //     panic!("Failed to setup framebuffer!");
        // }
        //
        graphics.gl.BindFramebuffer(gl::FRAMEBUFFER, fbo);
        graphics.gl.BindRenderbuffer(gl::RENDERBUFFER, rbo);
        graphics.gl.RenderbufferStorage(gl::RENDERBUFFER, gl::RGBA, width as GLsizei, height as GLsizei);
        graphics.gl.FramebufferRenderbuffer(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::RENDERBUFFER,
            rbo,
        );
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
        graphics.gl.Viewport(0, 0, width as GLsizei, height as GLsizei);
        check_for_errors(&graphics.gl);

        println!("Assigned size to FBO");
    }

    let program = unsafe { GeneralProgram::build(&graphics.gl).unwrap() };
    println!("Built shader program");

    // let (model_uniform, view_uniform, projection_uniform) = unsafe {
    //     let model = graphics.gl.GetUniformLocation(program.program, c_str!("model").as_ptr());
    //     let view = graphics.gl.GetUniformLocation(program.program, c_str!("view").as_ptr());
    //     let projection = graphics.gl.GetUniformLocation(program.program, c_str!("projection").as_ptr());
    //     (model, view, projection)
    // };

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
        graphics.gl.Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        check_for_errors(&graphics.gl);


        graphics.gl.DepthFunc(gl::LESS);
        check_for_errors(&graphics.gl);
    }
    println!("Enabled program and cleared buffer");


    // let program = Program::from_source(
    //     &display,
    //     include_str!("vert.glsl"),
    //     include_str!("frag.glsl"),
    //     None,
    // )?;

    // let mut target = display.draw();
    // target.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

    // let params = glium::DrawParameters {
    //     depth: glium::Depth {
    //         test: glium::DepthTest::IfLess,
    //         write: true,
    //         ..Default::default()
    //     },
    //     viewport: Some(glium::Rect { bottom: 0, left: 0, width, height }),
    //     ..Default::default()
    // };


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
            program.uniforms.set_light_direction(&graphics.gl, &light_direction);
            check_for_errors(&graphics.gl);
            // graphics.gl.UniformMatrix4fv(model_uniform, 1, gl::FALSE, building.transpose().as_ptr() as *const _);
            // graphics.gl.UniformMatrix4fv(view_uniform, 1, gl::FALSE, view.transpose().as_ptr() as *const _);
            // graphics.gl.UniformMatrix4fv(projection_uniform, 1, gl::FALSE, projection.transpose().as_ptr() as *const _);
        }

        // let uniforms = uniform! {
        //     model: building.data.0,
        //     view: view.data.0,
        //     projection: projection.data.0,
        // };

        for group in model.data.objects.iter().flat_map(|x| x.groups.iter()) {
            vertex_list.clear();
            vertex::vertex_buffer_for_group(&mut vertex_list, &model, group);
            // for SimplePolygon(polygon) in &group.polys {
            //     let initial = build_vertex(&model.data, polygon[0]);
            //     let mut prev = build_vertex(&model.data, polygon[1]);
            //
            //     for index in &polygon[2..] {
            //         let next = build_vertex(&model.data, *index);
            //
            //         vertex_list.push(initial);
            //         vertex_list.push(prev);
            //         vertex_list.push(next);
            //         prev = next;
            //     }
            // }

            unsafe {
                graphics.gl.BindVertexArray(vao);

                let vbo = load_vbo(&graphics.gl, &vertex_list);

                graphics.gl.BindBuffer(gl::ARRAY_BUFFER, vbo);

                Vertex::configure_vao(&graphics.gl, program.program);


                graphics.gl.DrawArrays(gl::TRIANGLES, 0, vertex_list.len() as GLsizei);

                graphics.gl.DeleteBuffers(1, &vbo as *const _);
                check_for_errors(&graphics.gl);

            }


            // let vbo = VertexBuffer::new(&display, &vertex_list)?;
            //
            // target.draw(&vbo, NoIndices(TrianglesList), &program, &uniforms, &params)?;
        }
    }

    println!("Rendered all buildings");

    let mut buffer = vec![0u8; (width * height * 3) as usize];

    unsafe {
        graphics.gl.Finish();
        println!("Finished Rendering");
        check_for_errors(&graphics.gl);

        println!("Reading pixels to buffer");
        graphics.gl.ReadPixels(0, 0, width as GLsizei, height as GLsizei, gl::RGB, gl::UNSIGNED_BYTE, buffer.as_mut_ptr() as *mut _);
        // graphics.gl.Ge(0, 0, width as GLsizei, height as GLsizei, gl::RGBA, gl::UNSIGNED_BYTE, buffer.as_mut_ptr() as *mut _);
        check_for_errors(&graphics.gl);

        // graphics.gl.BindFramebuffer(gl::DRAW_FRAMEBUFFER, 0);
        // graphics.gl.BindRenderbuffer(gl::RENDERBUFFER, 0);
        // graphics.gl.DeleteFramebuffers(1, &fbo);
        // graphics.gl.DeleteRenderbuffers(1, &rbo);
    }

    println!("Freed rendering resources");


    // let mut png_encoder = Encoder::new(
    //     File::create("out_glutin.png").unwrap(),
    //     width as u32,
    //     height as u32,
    // );
    // png_encoder.set_depth(BitDepth::Eight);
    // png_encoder.set_color(ColorType::Rgb);
    // let mut png_writer = png_encoder
    //     .write_header()
    //     .unwrap()
    //     .into_stream_writer_with_size(width as usize * 3)
    //     .unwrap();
    //
    // // from the padded_buffer we write just the unpadded bytes into the image
    // for chunk in buffer.chunks(width as usize * 3).rev() {
    //     png_writer
    //         .write_all(&chunk[..width as usize * 3])
    //         .unwrap();
    // }
    // png_writer.finish().unwrap();

    let mut img = RgbImage::from_raw(width, height, buffer).unwrap();

    println!("Flipping image in memory");
    flip_vertical_in_place(&mut img);

    println!("Shrinking image by a factor of {}", FORCED_SAMPLE_MULTIPLIER);
    let out = resize(&img, width / FORCED_SAMPLE_MULTIPLIER, height / FORCED_SAMPLE_MULTIPLIER, FilterType::Triangle);

    println!("Saving image to disk");
    out.save("out_glutin.png").unwrap();

    println!("Wrote image!");


    // target.finish()?;
    //
    // // reading the front buffer into an image
    // let image: glium::texture::RawImage2d<'_, u8> = display.read_front_buffer()?;
    // let image =
    //     image::ImageBuffer::from_raw(image.width, image.height, image.data.into_owned()).unwrap();
    // let image = image::DynamicImage::ImageRgba8(image).flipv();
    // let image = image.resize(image.width(), image.height(), FilterType::Triangle);
    // image.save("blueprint_render.png").unwrap();

    Ok(())
}


// fn render_scene() {
//
//     unsafe {
//         // graphics.gl.Enable(gl::DEPTH_TEST);
//         graphics.gl.UseProgram(program.program);
//         graphics.gl.BindVertexArray(program.vao);
//
//         graphics.gl.ClearColor(0.1, 0.1, 0.1, 1.0);
//         graphics.gl.Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
//     }
//     println!("Enabled program and cleared buffer");
//
//
//     // let program = Program::from_source(
//     //     &display,
//     //     include_str!("vert.glsl"),
//     //     include_str!("frag.glsl"),
//     //     None,
//     // )?;
//
//     // let mut target = display.draw();
//     // target.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);
//
//     // let params = glium::DrawParameters {
//     //     depth: glium::Depth {
//     //         test: glium::DepthTest::IfLess,
//     //         write: true,
//     //         ..Default::default()
//     //     },
//     //     viewport: Some(glium::Rect { bottom: 0, left: 0, width, height }),
//     //     ..Default::default()
//     // };
//
//
//     let projection = perspective(width as f32 / height as f32, 1.2, 0.1, 100.0);
//
//     let view = look_at(
//         &Vec3::new(-10.0, 10.0, -10.0),
//         &Vec3::new(0.0, 0.0, 0.0),
//         &Vec3::new(0.0, 1.0, 0.0),
//     );
//
//     for entry in entries {
//         let model = match model_loader.load_model(&entry.internal_name) {
//             Model::Missing => continue,
//             Model::Resolved { model } => model,
//         };
//
//         let pos = translation(&Vec3::new(entry.x as f32, entry.l as f32, entry.y as f32));
//         let pos = nalgebra_glm::scale(&pos, &Vec3::new(1.0, 1.0, -1.0));
//         let building = rotate_y(&pos, entry.r as f32 * f32::PI() / 2.0);
//
//         unsafe {
//             graphics.gl.UniformMatrix4fv(model_uniform, 1, gl::FALSE, building.transpose().as_ptr() as *const _);
//             graphics.gl.UniformMatrix4fv(view_uniform, 1, gl::FALSE, view.transpose().as_ptr() as *const _);
//             graphics.gl.UniformMatrix4fv(projection_uniform, 1, gl::FALSE, projection.transpose().as_ptr() as *const _);
//         }
//         // let uniforms = uniform! {
//         //     model: building.data.0,
//         //     view: view.data.0,
//         //     projection: projection.data.0,
//         // };
//
//         for group in model.data.objects.iter().flat_map(|x| x.groups.iter()) {
//             let vertex_list = vertex::vertex_buffer_for_group(&model, group);
//             // for SimplePolygon(polygon) in &group.polys {
//             //     let initial = build_vertex(&model.data, polygon[0]);
//             //     let mut prev = build_vertex(&model.data, polygon[1]);
//             //
//             //     for index in &polygon[2..] {
//             //         let next = build_vertex(&model.data, *index);
//             //
//             //         vertex_list.push(initial);
//             //         vertex_list.push(prev);
//             //         vertex_list.push(next);
//             //         prev = next;
//             //     }
//             // }
//
//             unsafe {
//                 let vbo = load_vbo(&graphics.gl, &vertex_list);
//
//                 graphics.gl.BindBuffer(gl::ARRAY_BUFFER, vbo);
//                 graphics.gl.BindVertexArray(program.vao);
//                 graphics.gl.DrawArrays(gl::TRIANGLES, 0, vertex_list.len() as GLsizei);
//
//                 graphics.gl.DeleteBuffers(1, &vbo as *const _);
//             }
//
//
//             // let vbo = VertexBuffer::new(&display, &vertex_list)?;
//             //
//             // target.draw(&vbo, NoIndices(TrianglesList), &program, &uniforms, &params)?;
//         }
//     }
// }


pub fn load_vbo<T>(gl: &Gl, buffer: &[T]) -> GLuint {
    unsafe {
        let mut vbo = 0;
        gl.GenBuffers(1, &mut vbo);
        gl.BindBuffer(gl::ARRAY_BUFFER, vbo);

        gl.BufferData(gl::ARRAY_BUFFER, (size_of::<T>() * buffer.len()) as GLsizeiptr, buffer.as_ptr() as *const _, gl::STATIC_DRAW);

        vbo
    }
}


#[track_caller]
pub fn check_for_errors(gl: &Gl) {
    let mut has_errored = false;
    loop {
        let mut err = unsafe { gl.GetError() };
        has_errored |= err != gl::NO_ERROR;

        match err {
            gl::NO_ERROR => {
                if has_errored {
                    panic!("Exited due to previous error")
                }
                return
            },
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


