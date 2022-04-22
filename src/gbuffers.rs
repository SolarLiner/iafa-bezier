use glutin::dpi::PhysicalSize;

use violette_low::{
    base::bindable::BindableExt,
    framebuffer::{BoundFB, ClearBuffer, Framebuffer},
    texture::{DepthStencil, Dimension, SampleMode, Texture, TextureUnit},
};

use crate::screen_draw::ScreenDraw;

pub struct GeometryBuffers {
    screen_pass: ScreenDraw,
    gfbo: Framebuffer,
    gcolor: Texture<[f32; 4]>,
    gdepth: Texture<DepthStencil<f32, ()>>,
    exposure: f32,
}

impl GeometryBuffers {
    pub fn new(size: PhysicalSize<u32>) -> anyhow::Result<Self> {
        let mut gcolor = Texture::new(size.width, size.height, 1, Dimension::D2);
        gcolor.with_binding(|tex| {
            tex.filter_min(SampleMode::Linear)?;
            tex.filter_mag(SampleMode::Linear)?;
            tex.reserve_memory()
        })?;

        let mut gdepth = Texture::new(size.width, size.height, 1, Dimension::D2);
        gdepth.with_binding(|tex| {
            tex.filter_min(SampleMode::Linear)?;
            tex.filter_mag(SampleMode::Linear)?;
            tex.reserve_memory()
        })?;

        let mut gfbo = Framebuffer::new();
        gfbo.with_binding(|fbo| {
            fbo.attach_color(0, &gcolor)?;
            fbo.attach_depth(&gdepth)?;
            fbo.assert_complete()
        })?;
        Ok(Self {
            gfbo,
            gcolor,
            gdepth,
            screen_pass: ScreenDraw::load("assets/shaders/screen/tonemapping.glsl")?,
            exposure: 1.,
        })
    }

    pub fn set_exposure(&mut self, v: f32) {
        self.exposure = v;
    }

    pub fn framebuffer(&mut self) -> &mut Framebuffer {
        &mut self.gfbo
    }

    pub fn draw(&mut self, frame: &mut BoundFB) -> anyhow::Result<()> {
        frame.clear_depth(1.0);
        frame.do_clear(ClearBuffer::COLOR | ClearBuffer::DEPTH)?;

        let unit = TextureUnit(0);
        self.screen_pass
            .with_uniform("in_color", |loc| loc.set(unit))?;
        self.screen_pass
            .with_uniform("exposure", |loc| loc.set(self.exposure))?;
        self.gcolor.set_texture_unit(unit);

        let _gcoltex = self.gcolor.bind()?;
        self.screen_pass.draw(frame)
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) -> anyhow::Result<()> {
        self.gfbo
            .bind()?
            .viewport(0, 0, size.width as _, size.height as _);
        self.gcolor
            .bind()?
            .clear_resize(size.width, size.height, 1)?;
        self.gdepth
            .bind()?
            .clear_resize(size.width, size.height, 1)?;
        Ok(())
    }
}
