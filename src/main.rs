use std::time::Instant;

use anyhow::{Context, Error};
use bytemuck::{Pod, Zeroable};
use glam::{vec3, Vec3};
use glutin::{
    dpi::PhysicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    ContextBuilder,
};
use tracing::Level;
use tracing_subscriber::EnvFilter;

use violette_low::{
    base::bindable::BindableExt,
    buffer::{Buffer, BufferKind},
    framebuffer::{ClearBuffer, Framebuffer},
    program::{Linked, Program},
    texture::{Texture, TextureUnit},
    vertex::{AsVertexAttributes, DrawMode, VertexArray},
};

use crate::camera::Camera;
use crate::material::Material;
use crate::mesh::Mesh;
use crate::transform::Transform;

mod camera;
mod material;
mod mesh;
mod transform;

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C, packed)]
struct Vertex {
    pos: [f32; 2],
    color: [f32; 3],
    uv: [f32; 2],
}

impl AsVertexAttributes for Vertex {
    type Attr = ([f32; 2], [f32; 3], [f32; 2]);
}

impl Vertex {
    const fn new(pos: [f32; 2], color: [f32; 3], uv: [f32; 2]) -> Self {
        Self { pos, color, uv }
    }
}

/*#[rustfmt::skip]
const VERTICES: [Vertex; 4] = [
    Vertex::new([ 0.5,  0.5], [1.0, 0.0, 0.0], [1.0, 1.0]),
    Vertex::new([ 0.5, -0.5], [0.0, 1.0, 0.0], [1.0, 0.0]),
    Vertex::new([-0.5, -0.5], [0.0, 0.0, 1.0], [0.0, 0.0]),
    Vertex::new([-0.5,  0.5], [1.0, 0.0, 1.0], [0.0, 1.0]),
];

#[rustfmt::skip]
const INDICES: [u32; 6] = [
    0, 1, 3,
    1, 2, 3,
];
*/
struct App {
    camera: Camera,
    mesh: Mesh,
    material: Material,
}

impl App {
    #[tracing::instrument]
    pub fn new() -> anyhow::Result<Self> {
        let mesh = Mesh::uv_sphere(1.0, 32, 32)?;
        let material = Material::create()?;
        let camera = Camera {
            transform: Transform::translation(vec3(0., 1., -4.)).looking_at(Vec3::ZERO),
            ..Default::default()
        };
        Ok(Self {
            camera,
            mesh,
            material,
        })
    }

    pub fn resize(&self, size: PhysicalSize<u32>) {
        Framebuffer::backbuffer()
            .bind()
            .unwrap()
            .viewport(0, 0, size.width as _, size.height as _);
    }

    #[tracing::instrument(skip_all)]
    pub fn render(&mut self) {
        let mut backbuffer = Framebuffer::backbuffer();
        let mut framebuffer = backbuffer.bind().unwrap();
        framebuffer.clear_color([0.1, 0.2, 0.2, 1.0]);
        framebuffer.do_clear(ClearBuffer::COLOR);

        match self.material
            .draw_mesh(&mut framebuffer, &self.camera, &mut self.mesh)
            .context("Cannot draw mesh on material") {
            Ok(()) => {}
            Err(err) => {tracing::warn!("Silenced error: {}", err)}
        }
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(EnvFilter::from_default_env())
        // .with_span_events(FmtSpan::ENTER | FmtSpan::EXIT)
        .init();
    let event_loop = EventLoop::new();
    let wb = WindowBuilder::new();
    let context = ContextBuilder::new()
        .build_windowed(wb, &event_loop)
        .context("Cannot create context")?;
    let context = unsafe { context.make_current().map_err(|(_, err)| err) }
        .context("Cannot create context")?;

    violette_low::load_with(|sym| context.get_proc_address(sym));
    violette_low::debug::set_message_callback(|data| {
        use violette_low::debug::CallbackSeverity::*;
        match data.severity {
            Notification => tracing::debug!(target: "gl", source=?data.source, message=%data.message, r#type=?data.r#type),
            Low => tracing::info!(target: "gl", source=?data.source, message=%data.message, r#type=?data.r#type),
            Medium => tracing::warn!(target: "gl", source=?data.source, message=%data.message, r#type=?data.r#type),
            High => tracing::error!(target: "gl", source=?data.source, message=%data.message, r#type=?data.r#type),
        };
    });

    let mut app = App::new().context("Cannot create application")?;

    event_loop.run(move |event, _, control_flow| {
        let next_frame_time = Instant::now() + std::time::Duration::from_nanos(16_666_667);
        *control_flow = ControlFlow::WaitUntil(next_frame_time);

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                WindowEvent::Resized(new_size) => {
                    context.resize(new_size);
                    app.resize(new_size);
                    context.window().request_redraw();
                }
                _ => return,
            },
            Event::NewEvents(cause) => match cause {
                StartCause::ResumeTimeReached { .. } => (),
                StartCause::Init => (),
                _ => *control_flow = ControlFlow::Poll,
            },
            _ => return,
        }
        app.render();
        context.swap_buffers().expect("Cannot swap buffers");
    });
}
