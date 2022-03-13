use anyhow::Context;
use glam::{vec2, vec3, Vec2, Vec3};

use violette_low::base::bindable::BindableExt;
use violette_low::buffer::BufferKind;
use violette_low::framebuffer::{BoundFB, Framebuffer};
use violette_low::vertex::DrawMode;
use violette_low::{
    buffer::Buffer,
    vertex::{AsVertexAttributes, VertexArray, VertexAttributes},
};
use crate::transform::Transform;

#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
}

impl AsVertexAttributes for Vertex {
    type Attr = (Vec3, Vec3, Vec2);
}

#[derive(Debug)]
pub struct Mesh {
    pub transform: Transform,
    array: VertexArray,
    indices: Buffer<u32>,
}

impl Mesh {
    pub fn uv_sphere(radius: f32, u_size: usize, v_size: usize) -> anyhow::Result<Self> {
        use std::f32::consts::*;
        let mut vertices = Vec::with_capacity(u_size * v_size + 2);
        let num_triangles = u_size * v_size * 2;
        let mut indices = Vec::with_capacity(num_triangles * 3);

        let lat_step = PI / v_size as f32;
        let lon_step = TAU / u_size as f32;

        vertices.push(Vertex {
            position: Vec3::Y,
            uv: vec2(0.5, 0.0),
            normal: Vec3::Y,
        });
        for j in 0..v_size {
            let phi = j as f32 * lon_step;
            for i in 0..u_size {
                let theta = i as f32 * lat_step;
                let (sphi, cphi) = phi.sin_cos();
                let sth = theta.sin();
                let normal = vec3(cphi * sth, sphi, cphi * sth);
                let position = normal * radius;
                let uv = vec2(phi / TAU, theta / PI);
                vertices.push(Vertex {
                    position,
                    normal,
                    uv,
                })
            }
        }

        // Indices: first row connected to north pole
        for i in 0..u_size {
            indices.extend([0, i + 1, i + 2])
        }

        // Triangles strips
        for j in 0..v_size - 1 {
            let row_start = j * u_size + 1;
            for i in 0..u_size {
                let first_corner = row_start + i;
                indices.extend([
                    first_corner,
                    first_corner + u_size + 1,
                    first_corner + u_size,
                ]);
            }
        }

        let indices = indices.into_iter().map(|i| i as u32).collect::<Vec<_>>();
        Ok(Self {
            transform: Transform::default(),
            array: {
                let mut vao = VertexArray::new();
                vao.with_binding(|vao| {
                    vao.with_vertex_buffer(Buffer::with_data(BufferKind::Array, &vertices)?)
                })?;
                vao
            },
            indices: Buffer::with_data(BufferKind::ElementArray, &indices)?,
        })
    }

    pub fn transformed(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    pub fn draw(&mut self, framebuffer: &mut BoundFB) -> anyhow::Result<()> {
        let _vaobind = self.array.bind()?;
        let ibuf_binding = self.indices.bind()?;
        framebuffer
            .draw_elements(DrawMode::TrianglesList, &ibuf_binding, ..)
            .context("Cannot draw mesh")?;
        Ok(())
    }
}
