use std::ops::{Deref, DerefMut};
use glam::{Vec2, Vec3};

#[derive(Debug, Clone)]
pub struct BezierCurve<V> {
    points: Vec<V>,
}

impl<V> Deref for BezierCurve<V> {
    type Target = [V];

    fn deref(&self) -> &Self::Target {
        self.points.deref()
    }
}

impl<V> DerefMut for BezierCurve<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.points.deref_mut()
    }
}

impl<V> BezierCurve<V> {
    pub fn new(data: impl IntoIterator<Item = V>) -> Self {
        Self {
            points: data.into_iter().collect(),
        }
    }
}

impl<V: Copy> BezierCurve<V> {
    pub fn get_point<F: Copy>(&self, s: F) -> V
    where
        V: Lerp<F>,
    {
        let mut points = self
            .points
            .windows(2)
            .map(|v| v[0].lerp(v[1], s))
            .collect::<Vec<_>>();
        if points.len() == 1 {
            points.remove(0)
        } else {
            Self { points }.get_point(s)
        }
    }
}

pub trait Lerp<F>: Sized {
    fn lerp(self, other: Self, s: F) -> Self;
}

impl Lerp<f32> for Vec2 {
    fn lerp(self, other: Self, s: f32) -> Self {
        Vec2::lerp(self, other, s)
    }
}

impl Lerp<f32> for Vec3 {
    fn lerp(self, other: Self, s: f32) -> Self {
        Vec3::lerp(self, other, s)
    }
}
