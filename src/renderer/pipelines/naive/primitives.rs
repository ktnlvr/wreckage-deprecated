extern crate nalgebra_glm as glm;
use glm::{vec3, Vec3};
use vulkano::buffer::BufferContents;

#[derive(Debug)]
pub struct Sphere {
    pub pos: glm::Vec3,
    pub radius: f32,
}

impl Sphere {
    pub fn new(pos: glm::Vec3, radius: f32) -> Self {
        Self { radius, pos }
    }

    pub fn raw(self) -> RawSphere {
        RawSphere {
            radius: self.radius,
            pos: [self.pos.x, self.pos.y, self.pos.z],
        }
    }

    pub fn bounds(&self) -> (Vec3, Vec3) {
        (
            vec3(self.radius, self.radius, self.radius) + self.pos,
            -vec3(self.radius, self.radius, self.radius) + self.pos,
        )
    }
}

#[repr(C)]
#[derive(Debug, BufferContents)]
pub struct RawSphere {
    pub pos: [f32; 3],
    pub radius: f32,
}
