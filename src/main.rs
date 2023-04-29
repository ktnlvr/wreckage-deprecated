use log::info;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use vulkano::VulkanLibrary;

fn main() {
    #[cfg(debug_assertions)]
    env_logger::builder().filter_level(log::LevelFilter::Trace).init();
    info!("Debug assertions & logging enabled");
    #[cfg(not(debug_assertions))]
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_resizable(false)
        .build(&event_loop)
        .unwrap();


    let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
    let extension_count = library.extension_properties().len();
    info!("{extension_count} extensions supported");

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            window_id,
        } if window_id == window.id() => *control_flow = ControlFlow::Exit,
        _ => (),
    });
}
