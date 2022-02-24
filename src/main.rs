use std::time::Instant;

use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use glutin::{
    dpi::PhysicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    ContextBuilder,
};
use tracing_subscriber::EnvFilter;
use violette_low::{
    base::bindable::BindableExt,
    buffer::{Buffer, BufferKind},
    framebuffer::{Backbuffer, ClearBuffer},
    program::{Linked, Program},
    texture::{Texture, TextureUnit},
    vertex::{AsVertexAttributes, DrawMode, VertexArray},
};

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

#[rustfmt::skip]
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

struct App {
    program: Program<Linked>,
    texture1: Texture<[u8; 3]>,
    texture2: Texture<[u8; 4]>,
    vertex_array: VertexArray,
    index_buffer: Buffer<u32>,
}

impl App {
    #[tracing::instrument]
    pub fn new() -> anyhow::Result<Self> {
        let mut program = Program::load(
            "assets/shaders/triangle.vert.glsl",
            Some("assets/shaders/triangle.frag.glsl"),
            None::<&'static str>,
        )
        .context("Cannot create shader program")?;
        program.validate()?;
        let vertex_buffer = Buffer::with_data(BufferKind::Array, &VERTICES)?;
        let index_buffer = Buffer::with_data(BufferKind::ElementArray, &INDICES)?;
        let mut vertex_array = VertexArray::new(DrawMode::TrianglesList);
        vertex_array.bind()?.with_vertex_buffer(vertex_buffer)?;
        let mut texture1 = Texture::from_image({
            let img = image::open("assets/textures/wall.jpg").context("Cannot load image file")?;
            img.to_rgb8()
        })
        .context("Cannot load texture")?;
        texture1.set_texture_unit(TextureUnit(0));

        let mut texture2 = Texture::from_image({
            image::open("assets/textures/awesomeface.png")
                .context("Cannot load image file")?
                .to_rgba8()
        })?;
        texture2.set_texture_unit(TextureUnit(1));

        program.with_binding(|p| {
            p.uniform("texture1").unwrap().set(TextureUnit(0))?;
            p.uniform("texture2").unwrap().set(TextureUnit(1))?;
            Ok(())
        })?;
        Ok(Self {
            program,
            texture1,
            texture2,
            vertex_array,
            index_buffer,
        })
    }

    pub fn resize(&self, size: PhysicalSize<u32>) {
        Backbuffer.viewport(0, 0, size.width as _, size.height as _);
    }

    #[tracing::instrument(skip_all)]
    pub fn render(&mut self) {
        let framebuffer = Backbuffer;
        framebuffer.clear_color([0.1, 0.2, 0.2]);
        framebuffer.clear(ClearBuffer::COLOR);

        let pbinding = self.program.bind().unwrap();
        let mut vbinding = self.vertex_array.bind().unwrap();
        let ibinding = self.index_buffer.bind().unwrap();
        let _tv1 = self.texture1.bind().unwrap();
        let _tv2 = self.texture2.bind().unwrap();
        vbinding.draw_indexed(&pbinding, &ibinding);
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
    violette_low::debug::set_message_callback(|debug| {
        eprintln!(
            "OpenGL {:?} {:?} ({:?}): {}",
            debug.source, debug.r#type, debug.severity, debug.message
        )
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
