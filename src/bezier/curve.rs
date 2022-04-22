use std::ops::{Deref, DerefMut};

use glam::{Vec2, Vec3};

#[derive(Debug, Clone)]
pub struct BezierCurve<V> {
    points: Vec<V>,
    looping: bool,
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
            looping: false,
        }
    }

    pub fn looping(mut self, v: bool) -> Self {
        self.looping = v;
        self
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
            Self {
                points,
                looping: false,
            }
            .get_point(s)
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

#[cfg(test)]
mod tests {
    use glam::{vec2, Vec2};
    use test_log::test;

    use super::BezierCurve;

    #[test]
    fn simple_curve() {
        let curve = BezierCurve::new([Vec2::ZERO, Vec2::X]);
        assert_eq!(curve.get_point(0.), Vec2::ZERO);
        assert_eq!(curve.get_point(1.), Vec2::X);
        assert_eq!(curve.get_point(0.5), vec2(0.5, 0.0));
    }
}
