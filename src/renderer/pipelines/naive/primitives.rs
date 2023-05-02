extern crate nalgebra_glm as glm;
use vulkano::buffer::BufferContents;

#[derive(Debug)]
pub struct Sphere {
    pub pos: glm::Vec3,
    pub radius: f32,
}

impl Sphere {
    pub fn new(pos: glm::Vec3, radius: f32, ) -> Self {
        Self { radius, pos }
    }

    pub fn raw(self) -> RawSphere {
        RawSphere {
            radius: self.radius,
            pos: [self.pos.x, self.pos.y, self.pos.z],
        }
    }
}

#[repr(C)]
#[derive(BufferContents)]
pub struct RawSphere {
    pub pos: [f32; 3],
    pub radius: f32,
}
