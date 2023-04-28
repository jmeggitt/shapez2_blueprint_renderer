pub mod context;
mod general;
pub mod gl;
mod shader;
mod util;
mod vertex;

use crate::blueprint::BlueprintEntry;
use crate::render::context::DisplayManager;
use crate::render::general::GeneralProgram;
use crate::render::gl::types::{GLsizei, GLuint};
use crate::render::util::{check_for_errors, load_vbo};
use crate::render::vertex::{vertex_buffer_for_model, Vertex};
use crate::tweaks::{Model, ModelLoader};
use crate::ARGS;
pub use context::setup_opengl;
pub use gl::Gl;
use glutin::config::{ColorBufferType, GlConfig};
use image::imageops::{flip_vertical_in_place, resize};
use image::{ImageFormat, RgbImage};
use log::{info, warn};
use nalgebra_glm::{
    look_at, perspective, rotate_y, rotation, scale, translate, translation, Mat4, Vec3, Vec4,
};
use num_traits::FloatConst;
use obj::Obj;
use std::collections::HashMap;
use std::io::{stdout, Cursor, Write};
use std::rc::Rc;
use std::time::{Duration, Instant};

pub fn perform_render(
    entries: &[BlueprintEntry],
    model_loader: &mut ModelLoader,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut ssaa = ARGS.ssaa.max(1);

    if ssaa > 16 {
        warn!("SSAA values over 16 will not have a noticeable effect on the image quality");
        ssaa = 16;
    }

    let (_, graphics) = setup_opengl(ARGS.width * ssaa, ARGS.height * ssaa);

    let color_sample_size = match graphics.gl_config.color_buffer_type() {
        Some(ColorBufferType::Rgb {
            r_size,
            g_size,
            b_size,
        }) => r_size.max(g_size).max(b_size),
        Some(ColorBufferType::Luminance(size)) => size,
        None => 8,
    };

    let color_buffer_resolution = 1u32 << color_sample_size;
    let msaa_samples = graphics.gl_config.num_samples().max(1);
    let max_useful_ssaa = (color_buffer_resolution as f64 / msaa_samples as f64)
        .sqrt()
        .ceil() as u32;

    // We have already created a window with the larger size, but we can still choose not to use
    // the entirety of the window for rendering with glViewport.
    if ssaa > max_useful_ssaa {
        warn!("The current SSAA setting combined with the system MSAA results in more samples being performed than the resolution of the color buffer. Reducing SSAA from {} to {}.", ssaa, max_useful_ssaa);
        ssaa = max_useful_ssaa;
    }

    if ssaa != 1 {
        info!(
            "Using SSAA to increase render samples by factor of {}",
            ssaa
        );
    }

    let mut render_width = ARGS.width * ssaa;
    let mut render_height = ARGS.height * ssaa;

    let window_size = graphics.window.inner_size();
    if render_width > window_size.width || render_height > window_size.height {
        warn!(
            "Window provided by system is not large enough to render with the output size and SSAA"
        );

        (render_width, render_height) = clamp_with_aspect_ratio(
            render_width,
            render_height,
            window_size.width,
            window_size.height,
        );
    }

    let mut img = unsafe {
        perform_render_impl(graphics, entries, model_loader, render_width, render_height)
    };

    if img.width() != ARGS.width || img.height() != ARGS.height {
        // TODO: Add CLI argument for resample type
        let resample_filter = ARGS.ssaa_sampler.0;

        info!(
            "Resampling image from render size ({}, {}) to desired size ({}, {}) using {:?} filter",
            img.width(),
            img.height(),
            ARGS.width,
            ARGS.height,
            resample_filter
        );

        let resize_start_time = Instant::now();
        img = resize(&img, ARGS.width, ARGS.height, resample_filter);
        info!(
            "Finished image resampling in {:?}",
            resize_start_time.elapsed()
        );
    }

    match ARGS.out_file.as_ref() {
        Some(path) => {
            info!("Saving result as {}", path.display());
            img.save(path)?;
        }
        None => {
            info!("Writing result to stdout");
            let mut buffer = Vec::with_capacity((img.width() * img.height() * 3) as usize);
            img.write_to(&mut Cursor::new(&mut buffer), ImageFormat::Png)?;

            let mut stdout = stdout().lock();
            stdout.write_all(&buffer)?;
            stdout.flush()?;
        }
    }

    Ok(())
}

fn clamp_with_aspect_ratio(
    src_width: u32,
    src_height: u32,
    dst_width: u32,
    dst_height: u32,
) -> (u32, u32) {
    if src_width <= dst_width && src_height <= dst_height {
        return (src_width, src_height);
    }

    let width = src_width * dst_height;
    let height = src_height * dst_width;

    if width > height {
        (dst_width, height / src_width)
    } else {
        (width / src_height, dst_height)
    }
}

unsafe fn perform_render_impl(
    graphics: DisplayManager,
    entries: &[BlueprintEntry],
    model_loader: &mut ModelLoader,
    width: u32,
    height: u32,
) -> RgbImage {
    let render_start_time = Instant::now();

    // Check that we actually have a buffer setup correctly
    if graphics.CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
        panic!("Failed to setup framebuffer!");
    }

    info!("Beginning render of size ({}, {})", width, height);
    graphics.Viewport(0, 0, width as GLsizei, height as GLsizei);
    check_for_errors(&graphics);

    let shader_compile_start_time = Instant::now();
    let program = unsafe { GeneralProgram::build(&graphics).unwrap() };
    info!(
        "Loaded and compiled shaders in {:?}",
        shader_compile_start_time.elapsed()
    );

    graphics.ClearColor(0.1, 0.1, 0.1, 1.0);
    graphics.Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

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
        2.0 * extension * (t + camera_space_aabb.max.z),
    );

    let camera_pos = -t * view_vector;
    let mut view = look_at(
        &(-t * view_vector),
        &Vec3::new(0.0, 0.0, 0.0),
        &Vec3::new(0.0, 1.0, 0.0),
    );

    if rotate_model {
        view *= rotation(f32::PI() / 2.0, &Vec3::new(0.0, 1.0, 0.0));
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

    info!("Waiting for completion of graphics render queue");
    graphics.Finish();
    info!(
        "Render completed. Total elapsed time to perform render: {:?}",
        render_start_time.elapsed()
    );

    info!("Performing call to glReadPixels to fetch image from graphics memory");

    let read_pixels_start_time = Instant::now();
    graphics.ReadPixels(
        0,
        0,
        width as GLsizei,
        height as GLsizei,
        gl::RGB,
        gl::UNSIGNED_BYTE,
        buffer.as_mut_ptr() as *mut _,
    );
    info!(
        "Completed call glReadPixels in {:?}",
        read_pixels_start_time.elapsed()
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
                warn!("Unable to invert camera matrix");
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
struct Aabb {
    min: Vec3,
    max: Vec3,
}

impl Aabb {
    pub fn expand_to_hold(&mut self, vertex: Vec3) {
        self.min.x = self.min.x.min(vertex.x);
        self.min.y = self.min.y.min(vertex.y);
        self.min.z = self.min.z.min(vertex.z);

        self.max.x = self.max.x.max(vertex.x);
        self.max.y = self.max.y.max(vertex.y);
        self.max.z = self.max.z.max(vertex.z);
    }

    pub fn expand_to_hold_aabb(&mut self, other: Aabb) {
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

    pub fn apply_transform(&self, transform: &Mat4) -> Aabb {
        let transformed_corners = self
            .corners()
            .map(|corner| (transform * Vec4::new(corner.x, corner.y, corner.z, 1.0)).xyz());

        let mut new_aabb = Aabb {
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
) -> (Vec<ModelGraphics>, Aabb) {
    let mut built_models: HashMap<*const Obj, (GLuint, GLsizei, Aabb)> =
        HashMap::with_capacity(entries.len());
    let mut models = Vec::with_capacity(entries.len());
    let mut aabb = Aabb::default();

    let mut model_vertex_buffer = Vec::new();

    let mut vertex_build_time = Duration::default();
    let mut aabb_build_time = Duration::default();
    let mut gpu_upload_time = Duration::default();

    // let blueprint_color = Vec3::new(0.18823, 0.51372, 0.86274);
    let blueprint_color = Vec3::new(56.0, 171.0, 203.0) / 255.0;

    for entry in entries {
        for Model { model, offset } in model_loader.load_model(entry.internal_name()) {
            let pos = translation(&entry.position());
            let pos = scale(&pos, &Vec3::new(1.0, 1.0, -1.0));
            let pos = rotate_y(&pos, entry.rotation());
            let pos = translate(&pos, offset);

            if let Some(&(vbo, vertex_count, model_aabb)) = built_models.get(&Rc::as_ptr(model)) {
                let aabb_build_start_time = Instant::now();
                aabb.expand_to_hold_aabb(model_aabb.apply_transform(&pos));
                aabb_build_time += aabb_build_start_time.elapsed();

                models.push(ModelGraphics {
                    vbo,
                    vertex_count,
                    model_uniform: pos,
                    color_uniform: blueprint_color,
                });

                continue;
            }

            let aabb_build_start_time = Instant::now();
            let mut model_aabb = Aabb::default();
            model
                .data
                .position
                .iter()
                .map(|array| Vec3::from(*array))
                .for_each(|vertex| model_aabb.expand_to_hold(vertex));

            aabb.expand_to_hold_aabb(model_aabb.apply_transform(&pos));
            aabb_build_time += aabb_build_start_time.elapsed();

            let vertex_build_start_time = Instant::now();
            model_vertex_buffer.clear();
            vertex_buffer_for_model(&mut model_vertex_buffer, model);
            vertex_build_time += vertex_build_start_time.elapsed();

            let vbo_creation_start_time = Instant::now();
            let vbo = load_vbo(gl, &model_vertex_buffer);
            gpu_upload_time += vbo_creation_start_time.elapsed();

            let vertex_count = model_vertex_buffer.len() as GLsizei;
            built_models.insert(Rc::as_ptr(model), (vbo, vertex_count, model_aabb));

            models.push(ModelGraphics {
                vbo,
                vertex_count,
                model_uniform: pos,
                color_uniform: blueprint_color,
            });
        }
    }

    info!("Sent model data to graphics memory:");
    info!("Vertex list build time: {:?}", vertex_build_time);
    info!("AABB build time: {:?}", aabb_build_time);
    info!("GL buffer upload time: {:?}", gpu_upload_time);

    (models, aabb)
}
