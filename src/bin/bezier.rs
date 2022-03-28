use std::{path::Path, time::Duration};

use glam::{vec2, Vec2};
use glutin::{dpi::PhysicalSize, event::WindowEvent};

use iafa_ig_projet::{bezier::curve::BezierCurve, run, Application};
use violette_low::buffer::BufferUsageHint;
use violette_low::vertex::VertexArray;
use violette_low::{
    base::bindable::BindableExt,
    buffer::{Buffer, BufferKind},
    framebuffer::Framebuffer,
    program::{Linked, Program},
    vertex::DrawMode,
};
use violette_low::framebuffer::ClearBuffer;

struct App {
    program: Program<Linked>,
    bezier: BezierCurve<Vec2>,
    vao: VertexArray,
}

impl Application for App {
    fn new(size: PhysicalSize<f32>) -> anyhow::Result<Self> {
        let program = Program::load(
            "assets/shaders/noop.vert.glsl",
            Some("assets/shaders/white.frag.glsl"),
            None::<&Path>,
        )?;
        let bezier = BezierCurve::new([vec2(-0.5, 0.5), vec2(0.75, 0.0), vec2(0.0, 1.0)]);
        let mut vao = VertexArray::new();
        vao.with_binding(|vao| {
            vao.with_vertex_buffer({
                let mut buf = Buffer::new(BufferKind::Array);
                let vertices = (0..100)
                    .map(|i| i as f32 / 100.)
                    .map(|s| bezier.get_point(s))
                    .collect::<Vec<_>>();
                buf.with_binding(|buf| buf.set(&vertices, BufferUsageHint::Dynamic))?;
                buf
            })
        })?;
        Ok(Self {
            program,
            bezier,
            vao,
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {}

    fn interact(&mut self, event: WindowEvent) {}

    fn tick(&mut self, dt: Duration) {
        let (s, c) = dt.as_secs_f32().sin_cos();
        self.bezier[0] = vec2(s, c);
    }

    fn render(&mut self) {
        let vertices = (0..100)
            .map(|i| i as f32 / 100.)
            .map(|s| self.bezier.get_point(s))
            .collect::<Vec<_>>();
        self.vao
            .buffer(0)
            .unwrap()
            .with_binding(|buf| buf.set(&vertices, BufferUsageHint::Dynamic))
            .unwrap();

        let mut bb = Framebuffer::backbuffer();
        let mut frame = bb.bind().unwrap();
        let mut vao_binding = self.vao.bind().unwrap();
        let mut _prog_binding = self.program.bind().unwrap();
        violette_low::set_line_width(1.);
        violette_low::set_line_smooth(true);
        frame.clear_color([0., 0., 0., 1.]);
        frame.do_clear(ClearBuffer::COLOR);
        frame
            .draw(
                &mut vao_binding,
                DrawMode::Wireframe,
                0..self.bezier.len() as i32,
            )
            .unwrap();
    }
}

fn main() -> anyhow::Result<()> {
    run::<App>("Bézier curve")
}
