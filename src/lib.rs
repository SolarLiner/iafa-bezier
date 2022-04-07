use std::fs::File;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::Context;
use glutin::event::{ElementState, ScanCode, VirtualKeyCode};
use glutin::window::Fullscreen;
use glutin::{
    dpi::PhysicalSize,
    event::Event,
    event::{KeyboardInput, StartCause, WindowEvent},
    event_loop::ControlFlow,
    event_loop::EventLoop,
    window::WindowBuilder,
    ContextBuilder,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub mod bezier;
pub mod camera;
pub mod light;
pub mod material;
pub mod mesh;
pub mod screen_draw;
pub mod transform;
pub mod gbuffers;

pub trait Application: Sized + Send + Sync {
    fn window_features(wb: WindowBuilder) -> WindowBuilder {
        wb
    }
    fn new(size: PhysicalSize<f32>) -> anyhow::Result<Self>;
    fn resize(&mut self, size: PhysicalSize<u32>);
    fn interact(&mut self, event: WindowEvent);
    /// /!\ Does not run on the main thread. OpenGL calls are unsafe here.
    fn tick(&mut self, dt: Duration);
    fn render(&mut self);
}

pub fn run<App: 'static + Application>(title: &str) -> anyhow::Result<()> {
    let fmt_layer = tracing_subscriber::fmt::Layer::default().pretty();
    let json_layer = tracing_subscriber::fmt::Layer::default()
        .json()
        .with_file(true)
        .with_level(true)
        .with_line_number(true)
        .with_thread_names(true)
        .with_thread_ids(true)
        .with_writer(File::create("log.json").unwrap());
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(EnvFilter::from_default_env())
        .with(json_layer)
        .init();
    let event_loop = EventLoop::new();
    let wb = App::window_features(WindowBuilder::new()).with_title(title);
    let context = ContextBuilder::new()
        //.with_gl_profile(glutin::GlProfile::Core)
        //.with_gl_debug_flag(true)
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
                let tick_start = Instant::now();
                app.lock().unwrap().tick(last_tick.elapsed());
                let tick_duration = tick_start.elapsed().as_secs_f32();
                last_tick = Instant::now();
                tracing::debug!(%tick_duration);
                std::thread::sleep(Duration::from_nanos(4_166_167)); // 240 FPS
            }
        }
    });

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
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::F11),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    if context.window().fullscreen().is_some() {
                        context.window().set_fullscreen(None)
                    } else {
                        context
                            .window()
                            .set_fullscreen(Some(Fullscreen::Borderless(None)))
                    }
                }
                WindowEvent::Resized(new_size) => {
                    context.resize(new_size);
                    app.lock().unwrap().resize(new_size);
                    context.window().request_redraw();
                }
                event => app.lock().unwrap().interact(event),
            },
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                context.window().request_redraw()
            }
            _ => {}
        }
    });
}
