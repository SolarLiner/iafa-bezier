use glam::Vec3;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(Debug, Copy, Clone, FromPrimitive)]
#[repr(u32)]
pub enum LightType {
    Point = 0,
    Directional = 1,
    Ambient = 2,
}

#[derive(Debug, Copy, Clone)]
pub enum Light {
    Point { color: Vec3, position: Vec3 },
    Directional { color: Vec3, dir: Vec3 },
    Ambient { color: Vec3 },
}

impl Light {
    fn pos_dir(&self) -> Vec3 {
        match self {
            &Self::Point { position, .. } => position,
            &Self::Directional { dir, .. } => dir,
            Self::Ambient { .. } => Vec3::ZERO,
        }
    }

    pub fn kind(&self) -> LightType {
        match self {
            Self::Point { .. } => LightType::Point,
            Self::Directional { .. } => LightType::Directional,
            Self::Ambient { .. } => LightType::Ambient,
        }
    }

    pub fn color(&self) -> Vec3 {
        match self {
            &Self::Directional { color, .. }
            | &Self::Point { color, .. }
            | &Self::Ambient { color } => color,
        }
    }

    pub fn color_mut(&mut self) -> &mut Vec3 {
        match self {
            Self::Directional { color, .. }
            | Self::Point { color, .. }
            | Self::Ambient { color } => color,
        }
    }
}

impl From<GpuLight> for Light {
    fn from(light: GpuLight) -> Self {
        let kind = LightType::from_u32(light.kind).unwrap();
        match kind {
            LightType::Point => Self::Point {
                position: light.pos_dir,
                color: light.color,
            },
            LightType::Directional => Self::Directional {
                dir: light.pos_dir,
                color: light.color,
            },
            LightType::Ambient => Self::Ambient { color: light.color },
        }
    }
}

#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C, packed)]
pub struct GpuLight {
    kind: u32,
    __pad0: [u8; 12],
    pos_dir: Vec3,
    __pad1: u32,
    color: Vec3,
    __pad2: [u8; 20],
}

impl From<Light> for GpuLight {
    fn from(l: Light) -> Self {
        Self {
            kind: l.kind() as _,
            pos_dir: l.pos_dir(),
            color: l.color(),
            __pad0: [0; 12],
            __pad1: 0,
            __pad2: [0; 20],
        }
    }
}
