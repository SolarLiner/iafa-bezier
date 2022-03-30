
use std::time::{Duration};

use anyhow::Context;

use glam::{vec3, Quat, Vec2, Vec3};
use glutin::event::{ElementState, MouseButton};
use glutin::{
    dpi::PhysicalSize,
    event::{WindowEvent},
};

use iafa_ig_projet::{
    Application,
    camera::{Camera, Projection},
    material::Material,
    mesh::Mesh,
    transform::Transform
};

use violette_low::{
    framebuffer::{DepthTestFunction, FramebufferFeature},
    base::bindable::BindableExt,
    framebuffer::{ClearBuffer, Framebuffer},
    texture::{Texture}
};

struct App {
    camera: Camera,
    mesh: Mesh,
    material: Material,
    dragging: bool,
    rot_target: Quat,
    last_mouse_pos: Vec2,
}

impl Application for App {
    #[tracing::instrument(target = "App::new")]
    fn new(size: PhysicalSize<f32>) -> anyhow::Result<Self> {
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
    fn resize(&mut self, size: PhysicalSize<u32>) {
        let camsize = size.cast();
        self.camera.projection.width = camsize.width;
        self.camera.projection.height = camsize.height;
        Framebuffer::backbuffer()
            .bind()
            .unwrap()
            .viewport(0, 0, size.width as _, size.height as _);
    }
    fn interact(&mut self, event: WindowEvent) {
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
    fn tick(&mut self, dt: Duration) {
        self.mesh.transform.rotation *= Quat::from_rotation_y(dt.as_secs_f32());
        self.camera.transform.rotation = self.camera.transform.rotation.lerp(self.rot_target, 1e-2);
    }
    #[tracing::instrument(target = "App::render", skip_all)]
    fn render(&mut self) {
        let mut backbuffer = Framebuffer::backbuffer();
        let mut framebuffer = backbuffer.bind().unwrap();
        framebuffer.clear_color([0.1, 0.1, 0.1, 1.0]);
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
    iafa_ig_projet::run::<App>("UV Sphere")
}