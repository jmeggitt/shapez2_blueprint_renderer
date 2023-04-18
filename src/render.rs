use crate::tweaks::{Model, ModelLoader};
use crate::BlueprintEntry;
use glium::glutin;
use glium::glutin::dpi::PhysicalSize;
use glium::glutin::window::WindowBuilder;
use glium::index::NoIndices;
use glium::index::PrimitiveType::TrianglesList;
use glium::Program;
use glium::{implement_vertex, uniform, Surface, VertexBuffer};
use image::imageops::FilterType;
use nalgebra_glm::{look_at, perspective, rotate_y, translation, Vec3};
use num_traits::float::FloatConst;
use obj::{IndexTuple, ObjData, SimplePolygon};

#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: [f32; 3],
    // texture: [f32; 2],
    normal: [f32; 3],
}

implement_vertex!(Vertex, position, normal);

pub fn perform_render(
    entries: &[BlueprintEntry],
    model_loader: &mut ModelLoader,
) -> Result<(), Box<dyn std::error::Error>> {
    // Render in 4k then scale it down by 4x to get better resolution
    let width = 3840;
    let height = 2160;

    // TODO: It should be possible to replace this with a simple framebuffer
    let event_loop = glutin::event_loop::EventLoop::new();
    let wb = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(width, height))
        .with_visible(false);
    let cb = glutin::ContextBuilder::new();
    let display = glium::Display::new(wb, cb, &event_loop).unwrap();

    let program = Program::from_source(
        &display,
        include_str!("vert.glsl"),
        include_str!("frag.glsl"),
        None,
    )?;

    let mut target = display.draw();
    target.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);

    let params = glium::DrawParameters {
        depth: glium::Depth {
            test: glium::DepthTest::IfLess,
            write: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut vertex_list: Vec<Vertex> = Vec::new();

    let projection = perspective(width as f32 / height as f32, 1.2, 0.1, 100.0);

    let view = look_at(
        &Vec3::new(-10.0, 10.0, -10.0),
        &Vec3::new(0.0, 0.0, 0.0),
        &Vec3::new(0.0, 1.0, 0.0),
    );

    for entry in entries {
        let model = match model_loader.load_model(&entry.internal_name) {
            Model::Missing => continue,
            Model::Resolved { model } => model,
        };

        let pos = translation(&Vec3::new(entry.x as f32, entry.l as f32, entry.y as f32));
        let building = rotate_y(&pos, entry.r as f32 * f32::PI() / 2.0);

        let uniforms = uniform! {
            model: building.data.0,
            view: view.data.0,
            projection: projection.data.0,
        };

        for group in model.data.objects.iter().flat_map(|x| x.groups.iter()) {
            vertex_list.clear();

            for SimplePolygon(polygon) in &group.polys {
                let initial = build_vertex(&model.data, polygon[0]);
                let mut prev = build_vertex(&model.data, polygon[1]);

                for index in &polygon[2..] {
                    let next = build_vertex(&model.data, *index);

                    vertex_list.push(initial);
                    vertex_list.push(prev);
                    vertex_list.push(next);
                    prev = next;
                }
            }

            let vbo = VertexBuffer::new(&display, &vertex_list)?;

            target.draw(&vbo, NoIndices(TrianglesList), &program, &uniforms, &params)?;
        }
    }

    target.finish()?;

    // reading the front buffer into an image
    let image: glium::texture::RawImage2d<'_, u8> = display.read_front_buffer()?;
    let image =
        image::ImageBuffer::from_raw(image.width, image.height, image.data.into_owned()).unwrap();
    let image = image::DynamicImage::ImageRgba8(image).flipv();
    let image = image.resize(image.width() / 4, image.height() / 4, FilterType::Triangle);
    image.save("blueprint_render.png").unwrap();

    Ok(())
}

fn build_vertex(data: &ObjData, index: IndexTuple) -> Vertex {
    match index {
        IndexTuple(pos, None, None) => Vertex {
            position: data.position[pos],
            // texture: [0.0; 2],
            normal: [0.0; 3],
        },
        IndexTuple(pos, Some(_text), None) => Vertex {
            position: data.position[pos],
            // texture: data.texture[text],
            normal: [0.0; 3],
        },
        IndexTuple(pos, Some(_text), Some(norm)) => Vertex {
            position: data.position[pos],
            // texture: data.texture[text],
            normal: data.normal[norm],
        },
        IndexTuple(pos, None, Some(norm)) => Vertex {
            position: data.position[pos],
            // texture: [0.0; 2],
            normal: data.normal[norm],
        },
    }
}
