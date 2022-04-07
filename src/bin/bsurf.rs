use std::time::Duration;

use anyhow::Context;
use glam::{vec3, Quat, Vec3};
use glutin::{dpi::PhysicalSize, event::WindowEvent};

use iafa_ig_projet::{
    bezier::{curve::BezierCurve, surface::BezierSurface},
    camera::{Camera, Projection},
    gbuffers::GeometryBuffers,
    light::{GpuLight, Light},
    material::Material,
    mesh::Mesh,
    run,
    screen_draw::ScreenDraw,
    transform::Transform,
    Application,
};
use violette_low::{
    base::bindable::BindableExt,
    buffer::{Buffer, BufferKind},
    framebuffer::{ClearBuffer, DepthTestFunction, Framebuffer, FramebufferFeature},
    texture::{Dimension, SampleMode, Texture, TextureUnit},
    Cull,
};

struct App {
    surface: BezierSurface,
    bezier_mesh: Option<Mesh>,
    lights: Buffer<GpuLight>,
    mat: Material,
    cam: Camera,
    screen_pass: GeometryBuffers,
}

fn cylinder() -> BezierSurface {
    BezierSurface::new([
        BezierCurve::new([
            vec3(1., 1., -1.),
            vec3(1., 1., 0.),
            vec3(1., 1., 1.),
            vec3(0., 1., 1.),
            vec3(-1., 1., 1.),
            vec3(-1., 1., 0.),
            vec3(-1., 1., -1.),
            vec3(0., 1., -1.),
            vec3(1., 1., -1.),
        ])
        .looping(true),
        BezierCurve::new([
            vec3(1., -1., -1.),
            vec3(1., -1., 0.),
            vec3(1., -1., 1.),
            vec3(0., -1., 1.),
            vec3(-1., -1., 1.),
            vec3(-1., -1., 0.),
            vec3(-1., -1., -1.),
            vec3(0., -1., -1.),
            vec3(1., -1., -1.),
        ])
        .looping(true),
    ])
}

impl Application for App {
    fn new(size: PhysicalSize<f32>) -> anyhow::Result<Self> {
        tracing::info!(?size);
        {
            let size = size.cast::<u32>();
            tracing::info!(?size);
            Framebuffer::backbuffer()
                .bind()?
                .viewport(0, 0, size.width as _, size.height as _);
        }
        let mut screen_pass = GeometryBuffers::new(size.cast())?;
        screen_pass.set_exposure(1.);
        violette_low::culling(Some(Cull::Back));

        Ok(Self {
            surface: cylinder(),
            bezier_mesh: None,
            lights: Buffer::with_data(
                BufferKind::Uniform,
                &[
                    Light::Directional {
                        color: vec3(2.5, 2.6, 2.1),
                        dir: Vec3::ONE.normalize(),
                    },
                    Light::Directional {
                        color: vec3(0.7, 0.9, 1.5),
                        dir: vec3(-1., 1., -1.).normalize(),
                    },
                ]
                .map(GpuLight::from),
            )?,
            mat: Material::create(
                Texture::from_image(image::open("assets/textures/moon_color.jpg")?.into_rgb32f())?,
                Texture::from_image(image::open("assets/textures/moon_normal.png")?.into_rgb32f())?,
            )?,
            cam: Camera {
                transform: Transform::translation(vec3(0., 0.5, -3.)).looking_at(Vec3::ZERO),
                projection: Projection {
                    width: size.width,
                    height: size.height,
                    zrange: 0.01..10.,
                    fovy: 45f32.to_radians(),
                },
            },
            screen_pass,
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        tracing::trace!(message = "Application resize", ?size);
        {
            let size = size.cast();
            self.cam.projection.width = size.width;
            self.cam.projection.height = size.height;
        }
        self.screen_pass.resize(size).unwrap();
        Framebuffer::backbuffer()
            .bind()
            .unwrap()
            .viewport(0, 0, size.width as _, size.height as _);
    }

    fn interact(&mut self, event: WindowEvent) {}

    fn tick(&mut self, dt: Duration) {
        self.cam.transform.rotation *= Quat::from_rotation_y(dt.as_secs_f32() * 0.4);
    }

    fn render(&mut self) {
        let mesh = self
            .bezier_mesh
            .get_or_insert_with(|| self.surface.triangulate(100, 100).unwrap());
        self.screen_pass
            .framebuffer()
            .with_binding(|frame| {
                frame.do_clear(ClearBuffer::COLOR)?;
                let mut lights = self.lights.bind()?;
                self.mat
                    .draw_mesh(frame, &self.cam, &mut *lights, std::array::from_mut(mesh))
            })
            .unwrap();

        Framebuffer::backbuffer()
            .with_binding(|frame| self.screen_pass.draw(frame))
            .unwrap();
    }
}

fn main() -> anyhow::Result<()> {
    run::<App>("Bézier Surface")
}
