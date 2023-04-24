pub mod context;
mod general;
pub mod gl;
mod shader;
mod vertex;

use crate::render::general::GeneralProgram;
use crate::render::gl::types::{GLsizei, GLsizeiptr, GLuint};
use crate::render::vertex::{vertex_buffer_for_model, Vertex};
use crate::tweaks::{Model, ModelLoader};
use crate::BlueprintEntry;
pub use context::setup_opengl;
pub use gl::Gl;
use image::imageops::{flip_vertical_in_place, resize, FilterType};
use image::RgbImage;
use nalgebra_glm::{
    look_at, perspective, rotate_y, rotation, scale, translation, Mat4, Vec3, Vec4,
};
use num_traits::FloatConst;
use std::collections::HashMap;
use std::mem::size_of;

const FORCED_SAMPLE_MULTIPLIER: u32 = 4;

pub fn perform_render(
    entries: &[BlueprintEntry],
    model_loader: &mut ModelLoader,
) -> Result<(), Box<dyn std::error::Error>> {
    let width = 1980;
    let height = 1080;

    let img = unsafe {
        perform_render_impl(
            entries,
            model_loader,
            width * FORCED_SAMPLE_MULTIPLIER,
            height * FORCED_SAMPLE_MULTIPLIER,
        )
    };

    println!(
        "Shrinking image by a factor of {}",
        FORCED_SAMPLE_MULTIPLIER
    );
    let out = resize(&img, width, height, FilterType::Triangle);

    println!("Saving image to disk");
    out.save("out_glutin.png").unwrap();

    println!("Wrote image!");
    Ok(())
}

unsafe fn perform_render_impl(
    entries: &[BlueprintEntry],
    model_loader: &mut ModelLoader,
    width: u32,
    height: u32,
) -> RgbImage {
    println!("Creating graphics context");
    let (_, graphics) = setup_opengl(width, height);

    // Check that we actually have a buffer setup correctly
    if graphics.CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
        panic!("Failed to setup framebuffer!");
    }

    // let mut fbo = 0;
    // let mut rbo = 0;
    // let mut texture = 0;
    // graphics.GenFramebuffers(1, &mut fbo as *mut _);
    // graphics.BindFramebuffer(gl::FRAMEBUFFER, fbo);
    // let mut framebuffer = 0;
    // let mut renderbuffer = 0;
    // graphics.GenFramebuffers(1, &mut fbo);
    // graphics.GenRenderbuffers(1, &mut rbo);
    println!("Created FBO and RBO");
    //
    // if graphics.CheckFramebufferStatus(fbo) != gl::FRAMEBUFFER_COMPLETE {
    //     panic!("Failed to setup framebuffer!");
    // }
    //
    // graphics.BindFramebuffer(gl::FRAMEBUFFER, fbo);
    // graphics.BindRenderbuffer(gl::RENDERBUFFER, rbo);
    // graphics.RenderbufferStorage(gl::RENDERBUFFER, gl::RGBA, width as GLsizei, height as GLsizei);
    // graphics.FramebufferRenderbuffer(
    //     gl::FRAMEBUFFER,
    //     gl::COLOR_ATTACHMENT0,
    //     gl::RENDERBUFFER,
    //     rbo,
    // );
    // check_for_errors(&graphics);

    // graphics.GenTextures(1, &mut texture);
    // graphics.BindTexture(gl::TEXTURE_2D, texture);
    //
    // graphics.TexImage2D(gl::TEXTURE_2D, 0, gl::RGB as GLint, width as GLsizei, height as GLsizei, 0, gl::RGB, gl::UNSIGNED_BYTE, 0 as *const c_void);
    //
    // graphics.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
    // graphics.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
    //
    // graphics.FramebufferTexture(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, texture, 0);
    //
    // let mut draw_buffers = gl::COLOR_ATTACHMENT0;
    // graphics.DrawBuffers(1, &mut draw_buffers);

    // graphics.BindFramebuffer(gl::FRAMEBUFFER, fbo);
    graphics
        .gl
        .Viewport(0, 0, width as GLsizei, height as GLsizei);
    check_for_errors(&graphics);

    println!("Assigned size to FBO");

    let program = unsafe { GeneralProgram::build(&graphics).unwrap() };
    println!("Built shader program");

    // graphics.Enable(gl::FRAMEBUFFE)
    graphics.ClearColor(0.1, 0.1, 0.1, 1.0);
    graphics
        .gl
        .Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

    graphics.Enable(gl::DEPTH_TEST);
    graphics.DepthFunc(gl::LESS);

    graphics.UseProgram(program.program);

    let mut vao = 0;
    graphics.GenVertexArrays(1, &mut vao);
    graphics.BindVertexArray(vao);

    let (mut models, mut aabb) = send_models_to_gpu(&graphics, entries, model_loader);

    let mut rotate_model = false;
    if aabb.max.x - aabb.min.x < aabb.max.z - aabb.min.z {
        rotate_model = true;

        aabb = aabb.apply_transform(&rotation(f32::PI() / 2.0, &Vec3::new(0.0, 1.0, 0.0)))
    }

    let aspect_ratio = width as f32 / height as f32;
    let fovy = 0.6;

    // projection
    let view_vector = Vec3::new(0.0, -1.0, 1.0).normalize();

    // Clip AABB to above ground level
    aabb.min.y = f32::max(aabb.min.y, 0.0);
    aabb.max.y = f32::max(aabb.max.y, 0.0);

    let camera_space_aabb = aabb.apply_transform(&look_at(
        &Vec3::new(0.0, 0.0, 0.0),
        &view_vector,
        &Vec3::new(0.0, 1.0, 0.0),
    ));

    let x_extent = f32::max(camera_space_aabb.min.x.abs(), camera_space_aabb.max.x);
    let y_extent = f32::max(camera_space_aabb.min.y.abs(), camera_space_aabb.max.y);

    let fovx = aspect_ratio * fovy;
    let min_x_t = x_extent / (fovx / 2.0).tan();
    let min_y_t = y_extent / (fovy / 2.0).tan();
    let t = 1.05 * (f32::max(min_x_t, min_y_t) + camera_space_aabb.min.z.abs());

    let extension = fovy.tan() / fovy.sin();
    let projection = perspective(
        aspect_ratio,
        fovy,
        0.1,
        extension * (t + camera_space_aabb.max.z),
    );

    let camera_pos = -t * view_vector;
    let mut view = look_at(
        &(-t * view_vector),
        &Vec3::new(0.0, 0.0, 0.0),
        &Vec3::new(0.0, 1.0, 0.0),
    );

    if rotate_model {
        view = view * rotation(f32::PI() / 2.0, &Vec3::new(0.0, 1.0, 0.0));
    }

    let light_direction = Vec3::new(1.0, -2.0, 1.0).normalize();

    program.uniforms.set_view(&graphics, &view);
    program.uniforms.set_projection(&graphics, &projection);
    program.uniforms.set_camera(&graphics, &camera_pos);
    program
        .uniforms
        .set_light_direction(&graphics, &light_direction);

    models.push(ModelGraphics::calculate_ground_plane(
        &graphics,
        projection * view,
    ));

    for model in &models {
        graphics.BindBuffer(gl::ARRAY_BUFFER, model.vbo);
        graphics.BindVertexArray(vao);
        Vertex::configure_vao(&graphics);

        program.uniforms.set_model(&graphics, &model.model_uniform);
        program
            .uniforms
            .set_material_color(&graphics, &model.color_uniform);

        graphics.DrawArrays(gl::TRIANGLES, 0, model.vertex_count);
    }

    let mut buffer = vec![0u8; (width * height * 3) as usize];

    println!("Wait for rendering to finish and extract pixels from framebuffer");
    graphics.Finish();
    graphics.ReadPixels(
        0,
        0,
        width as GLsizei,
        height as GLsizei,
        gl::RGB,
        gl::UNSIGNED_BYTE,
        buffer.as_mut_ptr() as *mut _,
    );

    check_for_errors(&graphics);
    match RgbImage::from_raw(width, height, buffer) {
        Some(mut img) => {
            flip_vertical_in_place(&mut img);
            img
        }
        None => unreachable!("Buffer was created with the correct size"),
    }
}

struct ModelGraphics {
    vbo: GLuint,
    vertex_count: GLsizei,
    model_uniform: Mat4,
    color_uniform: Vec3,
}

impl ModelGraphics {
    fn ground_plane_vertex(inverse_camera: &Mat4, viewport_x: f32, viewport_y: f32) -> Vertex {
        let frustum_near_corner = inverse_camera * Vec4::new(viewport_x, viewport_y, 0.0, 1.0);
        let frustum_far_corner = inverse_camera * Vec4::new(viewport_x, viewport_y, 1.0, 1.0);

        let frustum_near_corner = frustum_near_corner.xyz() / frustum_near_corner.w;
        let frustum_far_corner = frustum_far_corner.xyz() / frustum_far_corner.w;

        let offset = frustum_near_corner;
        let direction = (frustum_far_corner - frustum_near_corner).normalize();

        let ground_normal = Vec3::new(0.0, 1.0, 0.0);

        let denominator = ground_normal.dot(&direction);
        if denominator.abs() < f32::EPSILON {
            return Vertex::new(Vec3::default(), ground_normal);
        }

        let t = (-offset).dot(&ground_normal) / denominator;
        Vertex::new(offset + direction * t, ground_normal)
    }

    unsafe fn calculate_ground_plane(gl: &Gl, camera: Mat4) -> ModelGraphics {
        let buffer = match camera.try_inverse() {
            Some(inverse) => {
                let a = Self::ground_plane_vertex(&inverse, 1.0, 1.0);
                let b = Self::ground_plane_vertex(&inverse, -1.0, 1.0);
                let c = Self::ground_plane_vertex(&inverse, 1.0, -1.0);
                let d = Self::ground_plane_vertex(&inverse, -1.0, -1.0);

                [a, b, c, b, c, d]
            }
            None => {
                println!("Unable to invert camera matrix");
                let size = 1000.0;
                let ground_normal = Vec3::new(0.0, 1.0, 0.0);

                let a = Vertex::new(Vec3::new(size, 0.0, size), ground_normal);
                let b = Vertex::new(Vec3::new(-size, 0.0, size), ground_normal);
                let c = Vertex::new(Vec3::new(size, 0.0, -size), ground_normal);
                let d = Vertex::new(Vec3::new(-size, 0.0, -size), ground_normal);

                [a, b, c, b, c, d]
            }
        };

        ModelGraphics {
            vbo: load_vbo(gl, &buffer),
            vertex_count: 6,
            model_uniform: Mat4::identity(),
            color_uniform: Vec3::new(0.18039, 0.74902, 0.64706),
        }
    }
}

/// Axis Aligned Bounding Box
#[derive(Copy, Clone, Debug, Default)]
struct AABB {
    min: Vec3,
    max: Vec3,
}

impl AABB {
    pub fn expand_to_hold(&mut self, vertex: Vec3) {
        self.min.x = self.min.x.min(vertex.x);
        self.min.y = self.min.y.min(vertex.y);
        self.min.z = self.min.z.min(vertex.z);

        self.max.x = self.max.x.max(vertex.x);
        self.max.y = self.max.y.max(vertex.y);
        self.max.z = self.max.z.max(vertex.z);
    }

    pub fn expand_to_hold_aabb(&mut self, other: AABB) {
        self.expand_to_hold(other.min);
        self.expand_to_hold(other.max);
    }

    pub fn corners(&self) -> [Vec3; 8] {
        [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ]
    }

    pub fn apply_transform(&self, transform: &Mat4) -> AABB {
        let transformed_corners = self
            .corners()
            .map(|corner| (transform * Vec4::new(corner.x, corner.y, corner.z, 1.0)).xyz());

        let mut new_aabb = AABB {
            min: transformed_corners[0],
            max: transformed_corners[0],
        };

        transformed_corners
            .iter()
            .for_each(|corner| new_aabb.expand_to_hold(*corner));

        new_aabb
    }
}

unsafe fn send_models_to_gpu(
    gl: &Gl,
    entries: &[BlueprintEntry],
    model_loader: &mut ModelLoader,
) -> (Vec<ModelGraphics>, AABB) {
    let mut built_models: HashMap<String, (GLuint, GLsizei, AABB)> =
        HashMap::with_capacity(entries.len());
    let mut models = Vec::with_capacity(entries.len());
    let mut aabb = AABB::default();

    let mut model_vertex_buffer = Vec::new();

    for entry in entries {
        let model = match model_loader.load_model(&entry.internal_name) {
            Model::Missing => continue,
            Model::Resolved { model } => model,
        };

        let pos = translation(&Vec3::new(entry.x as f32, entry.l as f32, entry.y as f32));
        let pos = scale(&pos, &Vec3::new(1.0, 1.0, -1.0));
        let pos = rotate_y(&pos, entry.r as f32 * f32::PI() / 2.0);

        if let Some(&(vbo, vertex_count, model_aabb)) = built_models.get(&entry.internal_name) {
            aabb.expand_to_hold_aabb(model_aabb.apply_transform(&pos));

            models.push(ModelGraphics {
                vbo,
                vertex_count,
                model_uniform: pos,
                color_uniform: Vec3::new(0.18823, 0.51372, 0.86274),
            });

            continue;
        }

        let mut model_aabb = AABB::default();
        model
            .data
            .position
            .iter()
            .map(|array| Vec3::from(*array))
            .for_each(|vertex| model_aabb.expand_to_hold(vertex));

        aabb.expand_to_hold_aabb(model_aabb.apply_transform(&pos));

        model_vertex_buffer.clear();
        vertex_buffer_for_model(&mut model_vertex_buffer, model);

        let vbo = load_vbo(gl, &model_vertex_buffer);
        let vertex_count = model_vertex_buffer.len() as GLsizei;
        built_models.insert(
            entry.internal_name.to_owned(),
            (vbo, vertex_count, model_aabb),
        );

        models.push(ModelGraphics {
            vbo,
            vertex_count,
            model_uniform: pos,
            color_uniform: Vec3::new(0.18823, 0.51372, 0.86274),
        });
    }

    (models, aabb)
}

pub unsafe fn load_vbo<T>(gl: &Gl, buffer: &[T]) -> GLuint {
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
