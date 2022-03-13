use std::path::Path;
use violette_low::base::bindable::BindableExt;
use violette_low::framebuffer::{BoundFB};
use violette_low::program::{Linked, Program};
use violette_low::shader::{Shader, ShaderStage};
use crate::camera::Camera;
use crate::Mesh;

pub struct Material {
    program: Program<Linked>,
}

impl Material {
    pub fn create() -> anyhow::Result<Self> {
        let shaders_dir = Path::new("assets").join("shaders");
        let vert_shader = Shader::load(ShaderStage::Vertex, shaders_dir.join("mesh.vert.glsl"))?;
        let frag_shader = Shader::load(ShaderStage::Fragment, shaders_dir.join("mesh.frag.glsl"))?;
        let program = Program::from_shaders([vert_shader.id, frag_shader.id])?;
        Ok(Self { program })
    }

    pub fn draw_mesh(&mut self, framebuffer: &mut BoundFB, camera: &Camera, mesh: &mut Mesh) -> anyhow::Result<()> {
        let progbind = self.program.bind()?;
        progbind.uniform("model").unwrap().set(mesh.transform.matrix())?;
        progbind.uniform("view_proj").unwrap().set(camera.projection.matrix() * camera.transform.matrix())?;
        mesh.draw(framebuffer)?;
        Ok(())
    }
}
