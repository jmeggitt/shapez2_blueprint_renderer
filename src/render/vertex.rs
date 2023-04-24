use std::mem::size_of;
use obj::{Group, IndexTuple, Obj, ObjData, SimplePolygon};
use crate::{c_str, offset_of};
use crate::render::{check_for_errors, Gl, gl};
use crate::render::gl::types::{GLsizei, GLuint};

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Vertex {
    position: [f32; 3],
    // texture: [f32; 2],
    normal: [f32; 3],
}


impl Vertex {
    pub unsafe fn configure_vao(gl: &Gl, program: GLuint) {
        gl.VertexAttribPointer(
            0,
            3,
            gl::FLOAT,
            gl::FALSE,
            size_of::<Vertex>() as GLsizei,
            offset_of!(Vertex, position) as *const _,
        );
        gl.EnableVertexAttribArray(0);

        gl.VertexAttribPointer(
            1,
            3,
            gl::FLOAT,
            gl::FALSE,
            size_of::<Vertex>() as GLsizei,
            offset_of!(Vertex, normal) as *const _,
        );
        gl.EnableVertexAttribArray(1);
    }

    pub unsafe fn build_vao(gl: &Gl, program: GLuint) -> GLuint {
        let mut vao = 0;
        gl.GenVertexArrays(1, &mut vao);
        check_for_errors(&gl);

        gl.BindVertexArray(vao);
        check_for_errors(&gl);

        let position = gl.GetAttribLocation(program, c_str!("position").as_ptr()) as GLuint;
        check_for_errors(&gl);
        assert_eq!(position, 0);
        gl.VertexAttribPointer(
            position,
            3,
            gl::FLOAT,
            gl::FALSE,
            size_of::<Vertex>() as GLsizei,
            offset_of!(Vertex, position) as *const _,
        );
        check_for_errors(&gl);
        gl.EnableVertexAttribArray(position);
        check_for_errors(&gl);

        assert_eq!(6 * size_of::<f32>(), size_of::<Vertex>());
        assert_eq!(0, offset_of!(Vertex, position));
        assert_eq!(3 * size_of::<f32>(), offset_of!(Vertex, normal));

        let normal = gl.GetAttribLocation(program, c_str!("normal").as_ptr()) as GLuint;
        check_for_errors(&gl);
        assert_eq!(normal, 1);
        gl.VertexAttribPointer(
            normal,
            3,
            gl::FLOAT,
            gl::FALSE,
            size_of::<Vertex>() as GLsizei,
            offset_of!(Vertex, normal) as *const _,
        );
        check_for_errors(&gl);
        gl.EnableVertexAttribArray(normal);
        check_for_errors(&gl);

        vao
    }
}


pub fn build_vertex(data: &ObjData, index: IndexTuple) -> Vertex {
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


pub fn vertex_buffer_for_group(vertex_list: &mut Vec<Vertex>, model: &Obj, group: &Group) {
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
}

pub fn vertex_buffer_for_model(model: &Obj) -> Vec<Vertex> {
    let mut vertex_list = Vec::new();

    model.data.objects.iter()
        .flat_map(|object| object.groups.iter())
        .for_each(|group| vertex_buffer_for_group(&mut vertex_list, model, group));

    vertex_list
}
