use nalgebra_glm::{Mat4, Vec3};
use vulkano::buffer::BufferContents;

pub struct Camera {
    pub(crate) position: Vec3,
    pub(crate) rotation: Vec3,
}

#[derive(BufferContents)]
#[repr(C)]
pub struct RawCamera {
    position: [f32; 4],
    rotation_mat: [[f32; 4]; 4],
}

impl Camera {
    pub fn raw(&self) -> RawCamera {
        let mat = Mat4::from_euler_angles(self.rotation.x, self.rotation.y, self.rotation.z);
        RawCamera {
            position: [self.position.x, self.position.y, self.position.z, 0.0],
            rotation_mat: [
                [mat.m11, mat.m21, mat.m31, mat.m41],
                [mat.m12, mat.m22, mat.m32, mat.m42],
                [mat.m13, mat.m23, mat.m33, mat.m43],
                [mat.m14, mat.m24, mat.m34, mat.m44],
            ],
        }
    }
}
