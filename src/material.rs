use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use anyhow::Context;

use violette_low::base::bindable::BindableExt;
use violette_low::framebuffer::BoundFB;
use violette_low::program::{Linked, Program};
use violette_low::shader::{Shader, ShaderStage};
use violette_low::texture::{Texture, TextureUnit};

use crate::camera::Camera;
use crate::Mesh;

pub enum TextureSlot<const N: usize> {
    Texture(Texture<[f32; N]>),
    Color([f32; N]),
}

impl<const N: usize> From<Texture<[f32; N]>> for TextureSlot<N> {
    fn from(v: Texture<[f32; N]>) -> Self {
        Self::Texture(v)
    }
}

impl<const N: usize> From<[f32; N]> for TextureSlot<N> {
    fn from(v: [f32; N]) -> Self {
        Self::Color(v)
    }
}

impl<const N: usize> TextureSlot<N> {
    fn set_texture_unit(&mut self, unit: TextureUnit) {
        if let Self::Texture(tex) = self {
            tex.set_texture_unit(unit);
        }
    }

    fn unset_texture_unit(&mut self) {
        if let Self::Texture(tex) = self {
            tex.unset_texture_unit();
        }
    }
}

#[derive(Debug, Default)]
struct ShaderBuilder {
    sources: Vec<String>,
    defines: BTreeSet<String>,
    version_line: Option<String>,
}

impl ShaderBuilder {
    fn load<P: AsRef<Path>>(&mut self, path: P) -> anyhow::Result<()> {
        self.add_source(std::fs::read_to_string(path).context("I/O error")?)
    }

    fn add_source(&mut self, source: impl ToString) -> anyhow::Result<()> {
        const VERSION_STR: &str = "#version";
        let source = source.to_string();
        let mut lines =
            source
                .lines()
                .filter_map(|v| if !v.is_empty() { Some(v.trim()) } else { None });
        let first_line = lines.next().context("Empty source")?;
        let rest = lines.collect::<Vec<_>>().join("\n");
        if first_line.starts_with(VERSION_STR) {
            self.version_line.replace(first_line.to_string());
            self.sources.push(rest);
        } else {
            self.sources.push(first_line.to_string());
            self.sources.push(rest);
        }
        Ok(())
    }

    fn define(&mut self, name: impl ToString) {
        self.defines.insert(name.to_string());
    }

    fn build(self, stage: ShaderStage) -> anyhow::Result<Shader> {
        let source = self
            .version_line
            .into_iter()
            .chain(self.defines.into_iter().map(|v| format!("#define {}", v)))
            .chain(self.sources.into_iter().map(|v| v.to_string()))
            .reduce(|mut s, v| {
                s.push_str("\n\n");
                s.push_str(&v);
                s
            })
            .context("Empty sources")?;
        tracing::debug!(%source);
        Shader::new(stage, &source).context("Cannot compile shader")
    }
}

pub struct Material {
    program: Program<Linked>,
    color_slot: TextureSlot<3>,
}

impl Material {
    pub fn create(color_slot: impl Into<TextureSlot<3>>) -> anyhow::Result<Self> {
        let mut color_slot = color_slot.into();
        let shaders_dir = Path::new("assets").join("shaders");
        let vert_shader = Shader::load(ShaderStage::Vertex, shaders_dir.join("mesh.vert.glsl"))?;
        let frag_shader = {
            let mut builder = ShaderBuilder::default();
            if let TextureSlot::Texture(_) = &color_slot {
                builder.define("HAS_COLOR_TEXTURE");
            }
            builder.load(shaders_dir.join("mesh.frag.glsl"))?;
            builder
                .build(ShaderStage::Fragment)
                .context("Cannot build material shader")?
        };
        let mut program = Program::from_shaders([vert_shader.id, frag_shader.id])?;
        program.with_binding(|progbind| match color_slot {
            TextureSlot::Texture(_) => progbind.uniform("color").unwrap().set(TextureUnit(0)),
            TextureSlot::Color(col) => progbind.uniform("color").unwrap().set(col),
        })?;
        color_slot.set_texture_unit(TextureUnit(0));
        Ok(Self {
            program,
            color_slot,
        })
    }

    pub fn draw_mesh(
        &mut self,
        framebuffer: &mut BoundFB,
        camera: &Camera,
        mesh: &mut Mesh,
    ) -> anyhow::Result<()> {
        let progbind = self.program.bind()?;
        progbind
            .uniform("model")
            .unwrap()
            .set(mesh.transform.matrix())?;
        progbind
            .uniform("view_proj")
            .unwrap()
            .set(camera.projection.matrix() * camera.transform.matrix())?;
        let _texbind = if let TextureSlot::Texture(tex) = &mut self.color_slot {
            Some(tex.bind()?)
        } else {
            None
        };
        mesh.draw(framebuffer)?;
        Ok(())
    }
}
