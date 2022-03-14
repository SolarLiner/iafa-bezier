use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use glam::{vec3, Quat, Vec2, Vec3};
use glutin::event::{ElementState, MouseButton};
use glutin::{
    dpi::PhysicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    ContextBuilder,
};
use tracing_flame::FlameLayer;
use tracing_subscriber::{EnvFilter, Layer};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use violette_low::framebuffer::{DepthTestFunction, FramebufferFeature};
use violette_low::{
    base::bindable::BindableExt,
    buffer::{Buffer, BufferKind},
    framebuffer::{ClearBuffer, Framebuffer},
    program::{Linked, Program},
    texture::{Texture, TextureUnit},
    vertex::{AsVertexAttributes, DrawMode, VertexArray},
};

use crate::camera::{Camera, Projection};
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
    dragging: bool,
    rot_target: Quat,
    last_mouse_pos: Vec2,
}

impl App {
    #[tracing::instrument(target = "App::new")]
    pub fn new(size: PhysicalSize<f32>) -> anyhow::Result<Self> {
        let mesh = Mesh::uv_sphere(1.0, 32, 32)?;
        let material = Material::create(Texture::from_image(
            image::open("assets/textures/moon_color.jpg")?.into_rgb32f(),
        )?)?;
        let camera = Camera {
            transform: Transform::translation(vec3(0., -1., -4.)).looking_at(Vec3::ZERO),
            projection: Projection {
                width: size.width,
                height: size.height,
                ..Default::default()
            },
        };
        Framebuffer::backbuffer()
            .bind()?
            .enable_feature(FramebufferFeature::DepthTest(DepthTestFunction::Less))?;
        let rot_target = camera.transform.rotation;
        Ok(Self {
            camera,
            mesh,
            material,
            dragging: false,
            rot_target,
            last_mouse_pos: Vec2::ONE / 2.,
        })
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        let camsize = size.cast();
        self.camera.projection.width = camsize.width;
        self.camera.projection.height = camsize.height;
        Framebuffer::backbuffer()
            .bind()
            .unwrap()
            .viewport(0, 0, size.width as _, size.height as _);
    }

    pub fn interact(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let position = position.cast();
                let position = Vec2::new(position.x, position.y);
                if self.dragging {
                    let delta = position - self.last_mouse_pos;
                    let delta = delta * 0.01;
                    self.rot_target = Quat::from_rotation_y(delta.x)
                        * Quat::from_rotation_x(delta.y)
                        * self.rot_target;
                }
                self.last_mouse_pos = position;
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.dragging = state == ElementState::Pressed;
            }
            _ => {}
        }
    }

    #[tracing::instrument(target = "App::tick", skip(self))]
    pub fn tick(&mut self, dt: Duration) {
        self.mesh.transform.rotation *= Quat::from_rotation_y(dt.as_secs_f32());
        self.camera.transform.rotation = self.camera.transform.rotation.lerp(self.rot_target, 1e-2);
    }

    #[tracing::instrument(target = "App::render", skip_all)]
    pub fn render(&mut self) {
        let mut backbuffer = Framebuffer::backbuffer();
        let mut framebuffer = backbuffer.bind().unwrap();
        framebuffer.clear_color([0.1, 0.2, 0.2, 1.0]);
        framebuffer.clear_depth(1.0);
        framebuffer.do_clear(ClearBuffer::COLOR | ClearBuffer::DEPTH);

        match self
            .material
            .draw_mesh(&mut framebuffer, &self.camera, &mut self.mesh)
            .context("Cannot draw mesh on material")
        {
            Ok(()) => {}
            Err(err) => {
                tracing::warn!("Silenced error: {}", err)
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let (flame_layer, _guard) = FlameLayer::with_file("tracing.folded")?;
    let fmt_layer = tracing_subscriber::fmt::Layer::default();
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(EnvFilter::from_default_env())
        .with(flame_layer)
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
            Notification => {
                tracing::debug!(target: "gl", source=?data.source, message=%data.message, r#type=?data.r#type)
            }
            Low => {
                tracing::info!(target: "gl", source=?data.source, message=%data.message, r#type=?data.r#type)
            }
            Medium => {
                tracing::warn!(target: "gl", source=?data.source, message=%data.message, r#type=?data.r#type)
            }
            High => {
                tracing::error!(target: "gl", source=?data.source, message=%data.message, r#type=?data.r#type)
            }
        };
    });

    let app =
        App::new(context.window().inner_size().cast()).context("Cannot create application")?;
    let app = Arc::new(Mutex::new(app));

    std::thread::spawn({
        let app = app.clone();
        move || {
            let mut last_tick = Instant::now();
            loop {
                app.lock().unwrap().tick(last_tick.elapsed());
                last_tick = Instant::now();
                std::thread::sleep(Duration::from_nanos(16_666_667));
            }
        }
    });

    let _start = Instant::now();
    let mut last_tick = Instant::now();
    let mut next_frame_time = Instant::now() + std::time::Duration::from_nanos(16_666_667);
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(next_frame_time);

        match event {
            Event::RedrawRequested(_) => {
                let mut app = app.lock().unwrap();
                let frame_start = Instant::now();
                app.render();
                context.swap_buffers().unwrap();
                let frame_time = frame_start.elapsed().as_secs_f32();
                tracing::debug!(%frame_time);
                next_frame_time = frame_start + Duration::from_nanos(16_666_667);
                *control_flow = ControlFlow::WaitUntil(next_frame_time);
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                WindowEvent::Resized(new_size) => {
                    context.resize(new_size);
                    app.lock().unwrap().resize(new_size);
                    context.window().request_redraw();
                }
                event => app.lock().unwrap().interact(event),
            },
            Event::NewEvents(cause) => match cause {
                StartCause::ResumeTimeReached { .. } => context.window().request_redraw(),
                _ => {}
            },
            _ => {}
        }
        // app.render();
        // context.swap_buffers().expect("Cannot swap buffers");
        last_tick = Instant::now();
    });
}
