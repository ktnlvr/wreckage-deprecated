use std::sync::Arc;

use vulkano::{device::Device, shader::ShaderModule};

mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        src: r"
        #version 460

        layout(local_size_x = 64, local_size_y = 8, local_size_z = 1) in;
        
        layout(set = 0, binding = 0, rgba8) uniform writeonly image2D img;
        
        void main() {
            ivec2 pixel = ivec2(gl_GlobalInvocationID.xy);

            vec4 uv = vec4(
                gl_GlobalInvocationID.x / (gl_NumWorkGroups.x + 0.0), 
                gl_GlobalInvocationID.y / (gl_NumWorkGroups.y + 0.0),
                0.25, 1.0);

            imageStore(img, pixel, uv);
        }
        ",
    }
}

pub(crate) fn shader(device: Arc<Device>) -> Arc<ShaderModule> {
    cs::load(device.clone()).expect("failed to create shader module")
}
