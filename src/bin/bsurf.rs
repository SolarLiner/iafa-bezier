use std::time::Duration;

use anyhow::Context;
use glam::{vec3, Quat, Vec3};
use glutin::{dpi::PhysicalSize, event::WindowEvent};

use iafa_ig_projet::{
    bezier::{curve::BezierCurve, surface::BezierSurface},
    camera::{Camera, Projection},
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
    texture::{DepthStencil, Dimension, SampleMode, Texture, TextureUnit},
};

struct App {
    hdr_color: Texture<[f32; 4]>,
    hdr_depth: Texture<DepthStencil<f32, ()>>,
    hdr_fb: Framebuffer,
    tonemap_stage: ScreenDraw,
    surface: BezierSurface,
    bezier_mesh: Option<Mesh>,
    lights: Buffer<GpuLight>,
    mat: Material,
    cam: Camera,
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
        ]),
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
        ]),
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
        let mut hdr_color = Texture::new(size.width as _, size.height as _, 1, Dimension::D2);
        hdr_color.set_texture_unit(TextureUnit(0));
        let mut hdr_depth = Texture::new(size.width as _, size.height as _, 1, Dimension::D2);

        let mut hdr_fb = Framebuffer::new();
        hdr_fb.with_binding(|fb| {
            fb.enable_feature(FramebufferFeature::DepthTest(DepthTestFunction::Less))?;
            hdr_color
                .with_binding(|color_tex| {
                    color_tex.reserve_memory()?;
                    color_tex.filter_min(SampleMode::Linear)?;
                    color_tex.filter_mag(SampleMode::Linear)
                })
                .context("FB Color attachment failed")?;
            match hdr_depth
                .with_binding(|depth_tex| {
                    depth_tex.reserve_memory()?;
                    depth_tex.filter_min(SampleMode::Nearest)?;
                    depth_tex.filter_mag(SampleMode::Nearest)
                })
                .context("FB Depth attachment failed")
            {
                Ok(()) => {}
                Err(err) => tracing::warn!("Silenced error: {}", err),
            }
            fb.viewport(0, 0, size.width as _, size.height as _);
            fb.attach_color(0, &hdr_color)?;
            fb.attach_depth(&hdr_depth)?;
            fb.assert_complete()
        })?;
        let mut tonemap_stage = ScreenDraw::new(
            &std::fs::read_to_string("assets/shaders/screen/tonemapping.glsl")
                .context("Cannot read tonemapping shader")?,
        )
        .context("Cannot create tonemapping stage")?;
        tonemap_stage.with_uniform("in_color", |loc| loc.set(TextureUnit(0)))?;
        Ok(Self {
            hdr_color,
            hdr_depth,
            hdr_fb,
            tonemap_stage,
            surface: cylinder(),
            bezier_mesh: None,
            lights: Buffer::with_data(
                BufferKind::Uniform,
                &[
                    Light::Directional {
                        color: vec3(2.3, 2.8, 1.8),
                        dir: Vec3::ONE.normalize(),
                    },
                    Light::Directional {
                        color: vec3(0.7, 0.9, 1.2),
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
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        tracing::trace!(message = "Application resize", ?size);
        {
            let size = size.cast();
            self.cam.projection.width = size.width;
            self.cam.projection.height = size.height;
        }
        self.hdr_color
            .bind()
            .unwrap()
            .clear_resize(size.width, size.height, 1)
            .unwrap();
        match self
            .hdr_depth
            .bind()
            .unwrap()
            .clear_resize(size.width, size.height, 1)
        {
            Ok(()) => {}
            Err(err) => {
                tracing::warn!("Silenced error: {}", err)
            }
        }
        self.hdr_fb
            .bind()
            .unwrap()
            .viewport(0, 0, size.width as _, size.height as _);
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
        self.hdr_fb
            .with_binding(|frame| {
                frame.do_clear(ClearBuffer::COLOR | ClearBuffer::DEPTH)?;
                let mut lights = self.lights.bind()?;
                self.mat
                    .draw_mesh(frame, &self.cam, &mut *lights, std::array::from_mut(mesh))
            })
            .unwrap();

        Framebuffer::backbuffer()
            .with_binding(|frame| {
                let color_unit = TextureUnit(0);
                self.hdr_color.set_texture_unit(color_unit);
                let _hdr_coltgt = self.hdr_color.bind()?;
                self.tonemap_stage
                    .with_uniform("in_color", |loc| loc.set(color_unit))?;
                self.tonemap_stage.draw(frame)
            })
            .unwrap();
    }
}

fn main() -> anyhow::Result<()> {
    run::<App>("Bézier Surface")
}
