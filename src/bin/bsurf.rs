use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::time::Duration;

use anyhow::Context;
use glam::{vec3, Quat, Vec3};
use glutin::dpi::PhysicalSize;
use glutin::event::WindowEvent;
use glutin::window::{Fullscreen, WindowBuilder};

use iafa_ig_projet::{
    bezier::{
        curve::BezierCurve,
        surface::BezierSurface
    },
    camera::{Camera, Projection},
    light::{GpuLight, Light},
    material::Material,
    mesh::Mesh,
    screen_draw::ScreenDraw,
    transform::Transform,
    run,
    Application
};
use violette_low::{
    base::bindable::BindableExt,
    buffer::{Buffer, BufferKind},
    framebuffer::{ClearBuffer, DepthTestFunction, Framebuffer, FramebufferFeature},
    texture::{Dimension, Texture, TextureUnit}
};

struct App {
    hdr_target: Texture<[f32; 4]>,
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
        let mut hdr_target = Texture::new(size.width as _, size.height as _, 1, Dimension::D2);
        hdr_target.set_texture_unit(TextureUnit(0));
        let mut hdr_depth = ManuallyDrop::new(Texture::new(size.width as _, size.height as _, 1, Dimension::D2));


        let mut hdr_fb = Framebuffer::new();
        hdr_fb.with_binding(|fb| {
            fb.clear_color([0., 0., 0., 1.]);
            fb.clear_depth(1.0);
            fb.enable_feature(FramebufferFeature::DepthTest(DepthTestFunction::Less))?;
            let color_tex = hdr_target.bind()?;
            let depth_tex = hdr_depth.bind()?;
            fb.attach_color(0, color_tex.deref())
                .context("Cannot attach texture")?;
            fb.attach_depth::<f32>(depth_tex.deref()).context("Cannot attach texture")
        })?;

        let mut tonemap_stage = ScreenDraw::new(&std::fs::read_to_string(
            "assets/shaders/screen/tonemapping.glsl",
        ).context("Cannot read tonemapping shader")?).context("Cannot create tonemapping stage")?;
        tonemap_stage.with_uniform("in_color", |loc| loc.set(TextureUnit(0)))?;
        Ok(Self {
            hdr_target,
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
        {
            let size = size.cast();
            self.cam.projection.width = size.width;
            self.cam.projection.height = size.height;
        }
        self.hdr_target = Texture::new(size.width, size.height, 1, Dimension::D2);
        self.hdr_fb.with_binding(|fb| {
            fb.viewport(0, 0, size.width as _, size.height as _);
            Ok(())
        }).unwrap();
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
                frame.do_clear(ClearBuffer::all());
                let mut lights = self.lights.bind()?;
                self.mat.draw_mesh(frame, &self.cam, mesh, &mut *lights)
            })
            .unwrap();

        Framebuffer::backbuffer()
            .with_binding(|frame| {
                self.tonemap_stage
                    .with_uniform("in_color", |loc| loc.set(TextureUnit(0)))?;
                self.tonemap_stage.draw(frame)
            })
            .unwrap();
    }
}

fn main() -> anyhow::Result<()> {
    run::<App>("Bézier Surface")
}
