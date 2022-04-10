use std::collections::BTreeSet;
use std::path::Path;

use anyhow::Context;
use crevice::std140::AsStd140;

use violette_low::base::bindable::BindableExt;
use violette_low::buffer::BoundBuffer;
use violette_low::framebuffer::{Blend, BoundFB, ClearBuffer, FramebufferFeature};
use violette_low::program::{Linked, Program};
use violette_low::shader::{Shader, ShaderStage};
use violette_low::texture::{Texture, TextureUnit};

use crate::light::{BoundLightBuffer, GpuLight};
use crate::{camera::Camera, mesh::Mesh};

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
            .chain(self.sources.into_iter())
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
    normal_map: Option<Texture<[f32; 3]>>,
}

impl Material {
    pub fn create(
        color_slot: impl Into<TextureSlot<3>>,
        normal_map: impl Into<Option<Texture<[f32; 3]>>>,
    ) -> anyhow::Result<Self> {
        let mut color_slot = color_slot.into();
        let mut normal_map = normal_map.into();
        let shaders_dir = Path::new("assets").join("shaders");
        let vert_shader = Shader::load(ShaderStage::Vertex, shaders_dir.join("mesh.vert.glsl"))?;
        let frag_shader = {
            let mut builder = ShaderBuilder::default();
            if let TextureSlot::Texture(_) = &color_slot {
                builder.define("HAS_COLOR_TEXTURE");
            }
            if normal_map.is_some() {
                builder.define("HAS_NORMAL_TEXTURE");
            }
            builder.load(shaders_dir.join("mesh.frag.glsl"))?;
            builder
                .build(ShaderStage::Fragment)
                .context("Cannot build material shader")?
        };
        let mut program = Program::from_shaders([vert_shader.id, frag_shader.id])?;
        program.with_binding(|progbind| {
            match &mut color_slot {
                TextureSlot::Texture(tex) => {
                    let unit = TextureUnit(0);
                    progbind.uniform("color").unwrap().set(unit)?;
                    tex.set_texture_unit(unit);
                }
                TextureSlot::Color(col) => progbind.uniform("color").unwrap().set(*col)?,
            }
            if let Some(tex) = &mut normal_map {
                let unit = TextureUnit(1);
                progbind.uniform("normal_map").unwrap().set(unit)?;
                tex.set_texture_unit(unit);
            }
            Ok(())
        })?;
        Ok(Self {
            program,
            color_slot,
            normal_map,
        })
    }

    pub fn draw_mesh(
        &mut self,
        framebuffer: &mut BoundFB,
        camera: &Camera,
        lights: &mut BoundLightBuffer,
        meshes: &mut [Mesh],
    ) -> anyhow::Result<()> {
        framebuffer.enable_feature(FramebufferFeature::Blending(Blend::SrcAlpha, Blend::One))?; // Additive blending
        meshes.sort_by_cached_key(|m| m.distance_to_camera(camera));
        let progbind = self.program.bind()?;
        let mat_view_proj = camera.projection.matrix() * camera.transform.matrix();
        progbind.uniform("view_proj").unwrap().set(mat_view_proj)?;
        progbind
            .uniform("inv_view_proj")
            .unwrap()
            .set(mat_view_proj.inverse())?;
        for light_idx in 0..lights.len() {
            framebuffer.do_clear(ClearBuffer::DEPTH).unwrap();
            progbind
                .uniform_block("Light", 0)
                .unwrap()
                .bind_block(&lights.slice(light_idx..=light_idx))
                .unwrap();
            for mesh in &mut *meshes {
                progbind
                    .uniform("model")
                    .unwrap()
                    .set(mesh.transform.matrix())?;
                let _coltex = if let TextureSlot::Texture(tex) = &mut self.color_slot {
                    Some(tex.bind()?)
                } else {
                    None
                };
                let _normtex = if let Some(tex) = &mut self.normal_map {
                    Some(tex.bind()?)
                } else {
                    None
                };
                mesh.draw(framebuffer)?;
            }
        }
        Ok(())
    }
}
