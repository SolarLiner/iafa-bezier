use std::time::Duration;

use anyhow::Context;
use glam::{vec2, vec3, Quat, Vec3, Vec3Swizzles};
use glutin::{dpi::PhysicalSize, event::WindowEvent};

use iafa_ig_projet::light::LightBuffer;
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
    lights: LightBuffer,
    mat: Material,
    cam: Camera,
    screen_pass: GeometryBuffers,
}

fn bsurface() -> BezierSurface {
    BezierSurface::new([
        BezierCurve::new(
            [
                vec2(1., -1.),
                vec2(0.5, 0.),
                vec2(0., 1.5),
                vec2(-0.5, 2.),
                vec2(-1., 0.5),
            ]
            .map(|v| v.extend(-1.)),
        ),
        BezierCurve::new(
            [
                vec2(1., -0.5),
                vec2(0.5, 1.),
                vec2(0., 2.),
                vec2(-0.5, 1.),
                vec2(-1., 0.5),
            ]
            .map(|v| v.extend(0.)),
        ),
        BezierCurve::new(
            [
                vec2(1., 0.5),
                vec2(0.5, 1.5),
                vec2(0., 1.),
                vec2(-0.5, 0.5),
                vec2(-1., 0.),
            ]
            .map(|v| v.extend(1.)),
        ),
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
        screen_pass
            .framebuffer()
            .with_binding(|frame| {
                frame.clear_color([0., 0., 0., 1.]);
                frame
                    .enable_feature(FramebufferFeature::DepthTest(DepthTestFunction::Less))
            })?;
        screen_pass.set_exposure(0.6);
        violette_low::culling(Some(Cull::Back));

        Framebuffer::backbuffer()
            .with_binding(|frame| {
                frame.viewport(0, 0, size.width as _, size.height as _);
                frame.clear_depth(1.0);
                Ok(())
            })?;

        Ok(Self {
            surface: bsurface(),
            bezier_mesh: None,
            lights: GpuLight::create_buffer([
                Light::Directional {
                    color: vec3(2.5, 2.6, 2.1),
                    dir: vec3(0.3, 0.5, -1.).normalize(),
                },
                Light::Directional {
                    color: vec3(0.9, 1.1, 1.2),
                    dir: vec3(-1., 1., -1.).normalize(),
                },
                Light::Point {
                    position: vec3(-1., 0.5, 2.),
                    color: vec3(1., 0.5, 1.3) * 3.,
                },
                Light::Ambient {
                    color: Vec3::ONE * 0.2,
                },
            ])?,
            mat: Material::create(
                Texture::load_rgb32f("assets/textures/floor_color.jpg")?,
                Texture::load_rgb32f("assets/textures/floor_normal.png")?,
                Texture::load_rg32f("assets/textures/floor_rough_metal.png")?,
            )?.with_normal_amount(3.)?,
            cam: Camera {
                transform: Transform::translation(vec3(0., 3., -3.)).looking_at(Vec3::Y * 0.5),
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
        //self.cam.transform.rotation *= Quat::from_rotation_y(dt.as_secs_f32() * 0.4);
        if let Some(mesh) = &mut self.bezier_mesh {
            mesh.transform.rotation *= Quat::from_rotation_y(dt.as_secs_f32() * 0.4);
        }
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
            .with_binding(|frame| {
                frame.do_clear(ClearBuffer::DEPTH)?;
                self.screen_pass.draw(frame)
            })
            .unwrap();
    }
}

fn main() -> anyhow::Result<()> {
    run::<App>("Bézier Surface")
}
