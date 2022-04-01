use std::time::Duration;

use glam::{vec3, Quat, Vec3};
use glutin::dpi::PhysicalSize;
use glutin::event::WindowEvent;
use glutin::window::{Fullscreen, WindowBuilder};

use iafa_ig_projet::bezier::curve::BezierCurve;
use iafa_ig_projet::bezier::surface::BezierSurface;
use iafa_ig_projet::camera::{Camera, Projection};
use iafa_ig_projet::material::Material;
use iafa_ig_projet::mesh::Mesh;
use iafa_ig_projet::transform::Transform;
use iafa_ig_projet::{run, Application};
use violette_low::base::bindable::BindableExt;
use violette_low::framebuffer::{ClearBuffer, DepthTestFunction, Framebuffer, FramebufferFeature};
use violette_low::texture::Texture;

struct App {
    surface: BezierSurface,
    bezier_mesh: Option<Mesh>,
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
        Framebuffer::backbuffer().with_binding(|frame| {
            frame.clear_color([0., 0., 0., 1.]);
            frame.clear_depth(1.);
            frame.enable_feature(FramebufferFeature::DepthTest(DepthTestFunction::Less))
        })?;
        Ok(Self {
            surface: cylinder(),
            bezier_mesh: None,
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
        let mut bb = Framebuffer::backbuffer();
        let mut frame = bb.bind().unwrap();
        frame.do_clear(ClearBuffer::all());
        self.mat.draw_mesh(&mut frame, &self.cam, mesh).unwrap();
    }
}

fn main() -> anyhow::Result<()> {
    run::<App>("Bézier Surface")
}
