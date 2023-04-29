mod renderer;
use std::error::Error;

use renderer::pipelines::primitive::PrimitiveRenderer;

pub use renderer::prelude::*;

use log::info;
use vulkano::VulkanLibrary;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    #[cfg(debug_assertions)]
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .init();
    info!("Debug assertions & logging enabled");
    #[cfg(not(debug_assertions))]
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_resizable(false)
        .build(&event_loop)?;

    let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
    let extension_count = library.extension_properties().len();
    info!("{extension_count} extensions supported");

    let extensions = vulkano_win::required_extensions(&library);
    let primitive = PrimitiveRenderer::new(library, extensions)?;

    let mut renderer: Box<dyn Renderer> = Box::new(primitive);
    renderer.render_all();

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            window_id,
        } if window_id == window.id() => *control_flow = ControlFlow::Exit,
        _ => (),
    });
}
