use crate::render::gl::types::GLsizei;
use crate::render::{gl, Gl};
use memoffset::offset_of;
use nalgebra_glm::Vec3;
use obj::{Group, IndexTuple, Obj, ObjData, SimplePolygon};
use std::mem::size_of;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

impl Vertex {
    pub fn new(position: Vec3, normal: Vec3) -> Self {
        Vertex {
            position: position.data.0[0],
            normal: normal.data.0[0],
        }
    }

    pub unsafe fn configure_vao(gl: &Gl) {
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

pub fn vertex_buffer_for_model(vertex_list: &mut Vec<Vertex>, model: &Obj) {
    model
        .data
        .objects
        .iter()
        .flat_map(|object| object.groups.iter())
        .for_each(|group| vertex_buffer_for_group(vertex_list, model, group));
}
