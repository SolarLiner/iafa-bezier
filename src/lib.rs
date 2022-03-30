use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use std::fs::File;


use anyhow::Context;
use glutin::event::Event;
use glutin::event_loop::ControlFlow;
use glutin::{
    dpi::PhysicalSize,
    event::{StartCause, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
    ContextBuilder,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub mod camera;
pub mod material;
pub mod mesh;
pub mod transform;
pub mod screen_draw;
pub mod bezier;

pub trait Application: Sized + Send + Sync {
    fn new(size: PhysicalSize<f32>) -> anyhow::Result<Self>;
    fn resize(&mut self, size: PhysicalSize<u32>);
    fn interact(&mut self, event: WindowEvent);
    /// /!\ Does not run on the main thread. OpenGL calls are unsafe here.
    fn tick(&mut self, dt: Duration);
    fn render(&mut self);
}

pub fn run<App: 'static + Application>(title: &str) -> anyhow::Result<()> {
    let fmt_layer = tracing_subscriber::fmt::Layer::default();
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
    let wb = WindowBuilder::new().with_title(title);
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