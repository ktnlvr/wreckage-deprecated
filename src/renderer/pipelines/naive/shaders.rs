use std::sync::Arc;

use vulkano::{device::Device, shader::ShaderModule};

mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "./shaders/main.comp",
    }
}

pub(crate) fn shader(device: Arc<Device>) -> Arc<ShaderModule> {
    cs::load(device.clone()).expect("failed to create shader module")
}
