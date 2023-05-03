mod renderer;
use std::{error::Error, time};

use pipelines::naive::*;

pub use renderer::prelude::*;

use log::{debug, info};
use vulkano::{device::DeviceExtensions, VulkanLibrary};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::renderer::prelude::renderer::RenderingContext;

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    debug!(
        "Using commit {}",
        git_version::git_version!(fallback = "unknown")
    );

    let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
    let extension_count = library.extension_properties().len();
    info!("{extension_count} extensions supported");

    let instance_ext = vulkano_win::required_extensions(&library);
    let device_ext = DeviceExtensions {
        khr_swapchain: true,
        ..Default::default()
    };

    let ctx = RenderingContext::new(library, instance_ext, device_ext)?;

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_resizable(false)
        .build_vk_surface(&event_loop, ctx.instance.clone())?;

    let renderer = NaiveRenderer::new(ctx, window);

    let mut frame_begin = time::Instant::now();
    let mut fps_counter = 0;
    event_loop.run(move |event, _, control_flow| {
        match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        Event::WindowEvent {
            event: WindowEvent::Resized(_),
            ..
        } => {}
        Event::MainEventsCleared => {
            renderer.draw();
            let now = time::Instant::now();
            if now - frame_begin > time::Duration::new(1, 0) {
                frame_begin = now;
                debug!("FPS: {}", fps_counter);
                fps_counter = 0;
            }
            fps_counter += 1;
        }
        _ => (),
    }});
}
