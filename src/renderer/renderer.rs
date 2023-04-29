use std::sync::Arc;

use vulkano::instance::Instance;

pub trait Renderer: Send {
    fn instance(&self) -> Arc<Instance>;

    fn render_all(&mut self);
}
