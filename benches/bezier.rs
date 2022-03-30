use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use glam::Vec2;
use rand::prelude::*;

use iafa_ig_projet::bezier::curve::BezierCurve;

pub fn criterion_benchmark(c: &mut Criterion) {
    let points = (0..1000).map(|i| i as f32 / 1000.0).collect::<Vec<_>>();
    let mut rng = StdRng::seed_from_u64(42);
    let curves = [2usize, 3, 5, 10, 50, 100].map(|i| BezierCurve::<Vec2>::new((0..i).map(|_| rng.gen())));
    {
        let mut group = c.benchmark_group("Bezier curve");
        for curve in &curves {
            group.bench_with_input(
                BenchmarkId::from_parameter(curve.len()),
                curve,
                |b, curve| {
                    b.iter(|| {
                        points.iter().for_each(|&pt| {
                            curve.get_point(pt);
                        })
                    })
                },
            );
        }
        group.finish();
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
