use glam::{vec2, Vec3};

use crate::bezier::curve::BezierCurve;
use crate::mesh::{Mesh, Vertex};

pub struct BezierSurface {
    profile: Vec<BezierCurve<Vec3>>,
    dtprec: f32,
    looping: bool,
}

impl BezierSurface {
    pub fn new(profile: impl IntoIterator<Item = BezierCurve<Vec3>>) -> Self {
        Self {
            profile: profile.into_iter().collect(),
            dtprec: 1e-4,
            looping: false,
        }
    }

    pub fn with_precision(mut self, prec: f32) -> Self {
        self.dtprec = prec;
        self
    }

    pub fn looping(mut self, v: bool) -> Self {
        self.looping = v;
        self
    }

    pub fn get_point(&self, u: f32, v: f32) -> Vec3 {
        BezierCurve::new(self.profile.iter().map(|curve| curve.get_point(u)))
            .looping(self.looping)
            .get_point(v)
    }

    pub fn gradient(&self, u: f32, v: f32) -> Vec3 {
        let anchor = self.get_point(u, v);
        let du = (self.get_point(u + self.dtprec, v) - anchor) / self.dtprec;
        let dv = (self.get_point(u, v + self.dtprec) - anchor) / self.dtprec;
        du + dv
    }

    pub fn triangulate(&self, u: usize, v: usize) -> anyhow::Result<Mesh> {
        let mut vertices = Vec::with_capacity(u * v);
        for j in (0..v).map(|k| (k as f32 + 1.) / v as f32) {
            for i in (0..u).map(|k| (k as f32 + 1.) / u as f32) {
                let position = self.get_point(i, j);
                let normal = self.gradient(i, j).normalize();
                let uv = vec2(i, j);
                vertices.push(Vertex {
                    position,
                    normal,
                    uv,
                });
            }
        }

        let mut indices = Vec::with_capacity((u - 1) * (v - 1));
        for j in 0..v - 1 {
            for i in 0..u - 1 {
                let idx = j * u + i;
                let idx_next = idx + u;
                indices.extend([
                    /* face 1 */ idx,
                    idx + 1,
                    idx_next,
                    /* face 2 */ idx + 1,
                    idx_next + 1,
                    idx_next,
                ]);
            }
        }

        Mesh::new(vertices, indices.into_iter().map(|i| i as u32))
    }
}
