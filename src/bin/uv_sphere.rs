use std::time::Duration;

use anyhow::Context;
use glam::{vec3, Quat, Vec2, Vec3};
use glutin::{
    dpi::PhysicalSize,
    event::{ElementState, MouseButton, WindowEvent},
};

use iafa_ig_projet::light::LightBuffer;
use iafa_ig_projet::{
    camera::{Camera, Projection},
    gbuffers::GeometryBuffers,
    light::{GpuLight, Light},
    material::Material,
    mesh::Mesh,
    transform::Transform,
    Application,
};
use violette_low::{
    base::bindable::BindableExt,
    framebuffer::{ClearBuffer, DepthTestFunction, Framebuffer, FramebufferFeature},
    texture::Texture,
    Cull,
};

struct App {
    camera: Camera,
    mesh: Mesh,
    lights: LightBuffer,
    geom_pass: GeometryBuffers,
    material: Material,
    dragging: bool,
    rot_target: Quat,
    last_mouse_pos: Vec2,
}

impl Application for App {
    #[tracing::instrument(target = "App::new")]
    fn new(size: PhysicalSize<f32>) -> anyhow::Result<Self> {
        let mesh = Mesh::uv_sphere(1.0, 32, 32)?;
        let material = Material::create(
            Texture::from_image(image::open("assets/textures/moon_color.jpg")?.into_rgb32f())?,
            Texture::from_image(image::open("assets/textures/moon_normal.png")?.into_rgb32f())?,
            [0.8, 0.0],
        )?
        .with_normal_amount(0.2)?;
        let lights = GpuLight::create_buffer([
            Light::Directional {
                dir: Vec3::X,
                color: Vec3::ONE * 12.,
            },
            Light::Directional {
                dir: Vec3::Z,
                color: vec3(1., 1.5, 2.),
            },
        ])?;
        let camera = Camera {
            transform: Transform::translation(vec3(0., -1., -4.)).looking_at(Vec3::ZERO),
            projection: Projection {
                width: size.width,
                height: size.height,
                ..Default::default()
            },
        };
        let mut geom_pass = GeometryBuffers::new(size.cast())?;
        geom_pass.set_exposure(0.06);
        geom_pass
            .framebuffer()
            .bind()?
            .enable_feature(FramebufferFeature::DepthTest(DepthTestFunction::Less))?;
        let rot_target = camera.transform.rotation;
        violette_low::culling(Some(Cull::Back));

        Ok(Self {
            camera,
            mesh,
            lights,
            material,
            geom_pass,
            dragging: false,
            rot_target,
            last_mouse_pos: Vec2::ONE / 2.,
        })
    }
    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.camera.projection.update(size.cast());
        self.geom_pass.resize(size).unwrap();
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
        self.mesh.transform.rotation *= Quat::from_rotation_y(dt.as_secs_f32() * 0.1);
        self.camera.transform.rotation = self.camera.transform.rotation.lerp(self.rot_target, 1e-2);
    }
    #[tracing::instrument(target = "App::render", skip_all)]
    fn render(&mut self) {
        // Direct rendering
        /*
        Framebuffer::backbuffer().with_binding(|frame| {
            frame.clear_color([0., 0., 0., 1.]);
            frame.clear_depth(1.);
            frame.do_clear(ClearBuffer::COLOR | ClearBuffer::DEPTH)?;

            let mut lightbuf = self.lights.bind()?;
            self.material.draw_mesh(frame, &self.camera, &mut lightbuf, std::array::from_mut(&mut self.mesh))
        }).unwrap();
        */
        // 2-pass rendering
        self.geom_pass
            .framebuffer()
            .with_binding(|framebuffer| {
                framebuffer.clear_color([0., 0., 0., 1.0]);
                framebuffer.clear_depth(1.0);
                framebuffer.do_clear(ClearBuffer::COLOR | ClearBuffer::DEPTH)?;

                let mut lightbuf = self.lights.bind().unwrap();
                self.material
                    .draw_mesh(
                        framebuffer,
                        &self.camera,
                        &mut lightbuf,
                        std::array::from_mut(&mut self.mesh),
                    )
                    .context("Cannot draw mesh on material")
            })
            .unwrap();

        Framebuffer::backbuffer()
            .with_binding(|bb| {
                bb.clear_depth(1.0);
                bb.do_clear(ClearBuffer::DEPTH)?;
                self.geom_pass.draw(bb)
            })
            .unwrap();
    }
}

fn main() -> anyhow::Result<()> {
    iafa_ig_projet::run::<App>("UV Sphere")
}
