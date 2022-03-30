use std::cmp::Ordering;

use std::{path::Path, time::Duration};

use glam::{vec2, Vec2};
use glutin::dpi::PhysicalPosition;
use glutin::event::{ElementState, MouseButton};
use glutin::{dpi::PhysicalSize, event::WindowEvent};

use iafa_ig_projet::{bezier::curve::BezierCurve, run, Application};
use violette_low::buffer::BufferUsageHint;
use violette_low::framebuffer::ClearBuffer;
use violette_low::vertex::VertexArray;
use violette_low::{
    base::bindable::BindableExt,
    buffer::{Buffer, BufferKind},
    framebuffer::Framebuffer,
    program::{Linked, Program},
    vertex::DrawMode,
};

struct App {
    program: Program<Linked>,
    bezier: BezierCurve<Vec2>,
    vao: VertexArray,
    holding: Option<usize>,
    mouse_pos: Vec2,
    window_size: Vec2,
}

impl Application for App {
    fn new(size: PhysicalSize<f32>) -> anyhow::Result<Self> {
        let program = Program::load(
            "assets/shaders/noop.vert.glsl",
            Some("assets/shaders/color.frag.glsl"),
            None::<&Path>,
        )?;
        let bezier = BezierCurve::new((0..4).map(|_| 2. * rand::random::<Vec2>() - 1.));
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
            holding: None,
            mouse_pos: Default::default(),
            window_size: vec2(size.width, size.height),
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        let (w, h) = (size.width as _, size.height as _);
        let fsize = size.cast();
        self.window_size = vec2(fsize.width, fsize.height);
        Framebuffer::backbuffer()
            .bind()
            .unwrap()
            .viewport(0, 0, w, h);
    }

    fn interact(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let PhysicalPosition { x, y } = position.cast();
                let normalized_pos = vec2(x, y) / self.window_size;
                self.mouse_pos = 2. * normalized_pos - 1.;
                self.mouse_pos *= vec2(1., -1.);
                tracing::debug!(mouse_pos=?self.mouse_pos);
                if let Some(pos) = self.holding {
                    self.bezier[pos] = self.mouse_pos;
                }
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                if state == ElementState::Released {
                    self.holding.take();
                } else {
                    self.holding = self
                        .bezier
                        .iter()
                        .enumerate()
                        .min_by(|(_, l), (_, r)| {
                            l.distance_squared(self.mouse_pos)
                                .partial_cmp(&r.distance_squared(self.mouse_pos))
                                .unwrap_or(Ordering::Greater)
                        })
                        .map(|(i, _)| i);
                }
                tracing::debug!(?state, holding=?self.holding);
            }
            _ => {}
        }
    }

    fn tick(&mut self, _: Duration) {}

    fn render(&mut self) {
        let path = |s: f32| self.bezier.get_point(s);
        let vertices = (0..100)
            .map(|i| i as f32 / 100.)
            .map(path)
            .collect::<Vec<_>>();
        self.vao
            .buffer(0)
            .unwrap()
            .with_binding(|buf| buf.set(&vertices, BufferUsageHint::Dynamic))
            .unwrap();

        let mut bb = Framebuffer::backbuffer();
        let mut frame = bb.bind().unwrap();
        let prog_binding = self.program.bind().unwrap();
        let stroke_uniform = prog_binding.uniform("stroke").unwrap();
        violette_low::set_point_size(10.);
        violette_low::set_line_width(2.);
        violette_low::set_line_smooth(true);
        stroke_uniform.set([1f32, 1., 1.]).unwrap();
        frame.clear_color([0., 0., 0., 1.]);
        frame.do_clear(ClearBuffer::COLOR);
        self.vao
            .with_binding(|vao_binding| {
                frame.draw(vao_binding, DrawMode::LineStrip, 0..vertices.len() as i32)
            })
            .unwrap();

        stroke_uniform.set([1., 0., 1.]).unwrap();
        self.vao
            .buffer(0)
            .unwrap()
            .with_binding(|buf| buf.set(&self.bezier, BufferUsageHint::Dynamic))
            .unwrap();
        self.vao
            .with_binding(|vao_binding| {
                frame.draw(
                    vao_binding,
                    DrawMode::LineStrip,
                    0..self.bezier.len() as i32,
                )?;
                frame.draw(vao_binding, DrawMode::Points, 0..self.bezier.len() as i32)
            })
            .unwrap();
    }
}

fn main() -> anyhow::Result<()> {
    run::<App>("Bézier curve")
}
