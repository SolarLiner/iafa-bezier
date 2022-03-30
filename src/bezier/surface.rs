use glam::{vec2, Vec3};

use crate::bezier::curve::BezierCurve;
use crate::mesh::{Mesh, Vertex};

pub struct BezierSurface {
    profile: Vec<BezierCurve<Vec3>>,
}

impl BezierSurface {
    pub fn new(profile: impl IntoIterator<Item = BezierCurve<Vec3>>) -> Self {
        Self {
            profile: profile.into_iter().collect(),
        }
    }

    pub fn get_point(&self, u: f32, v: f32) -> Vec3 {
        BezierCurve::new(self.profile.iter().map(|curve| curve.get_point(u))).get_point(v)
    }

    pub fn get_derivative(&self, u: f32, v: f32) -> Vec3 {
        let anchor = self.get_point(u, v);
        let du = (self.get_point(u + 0.001, v) - anchor) * 1000.;
        let dv = (self.get_point(u, v + 0.001) - anchor) * 1000.;
        du + dv
    }

    pub fn get_normal(&self, u: f32, v: f32) -> Vec3 {
        let anchor = self.get_derivative(u, v);
        let ddu = (self.get_derivative(u + 0.001, v) - anchor) * 1000.;
        let ddv = (self.get_derivative(u, v + 0.001) - anchor) * 1000.;
        ddu + ddv
    }

    pub fn triangulate(&self, u: usize, v: usize) -> anyhow::Result<Mesh> {
        let mut vertices = Vec::with_capacity(u * v);
        for j in (0..v).map(|k| k as f32 / v as f32) {
            for i in (0..u).map(|k| k as f32 / u as f32) {
                let position = self.get_point(i, j);
                let normal = self.get_normal(i, j);
                let uv = vec2(i, j);
                vertices.push(Vertex {
                    position,
                    normal,
                    uv,
                });
            }
        }

        let mut indices = Vec::with_capacity((u - 1) * (v - 1));
        for j in 0..v-1 {
            for i in 0..u-1 {
                let idx = j * u + i;
                let idx_next = idx + u;
                indices.extend([
                    /* face 1 */ idx_next,
                    idx + 1,
                    idx,
                    /* face 2 */ idx_next,
                    idx_next + 1,
                    idx + 1,
                ]);
            }
        }

        Mesh::new(vertices, indices.into_iter().map(|i| i as u32))
    }
}