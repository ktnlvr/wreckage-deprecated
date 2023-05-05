mod renderer;
use std::{error::Error, time};

use nalgebra_glm::{euler, vec3, Mat4, Quat};
pub use renderer::prelude::*;

use log::{debug, info};
use vulkano::{device::DeviceExtensions, VulkanLibrary};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use crate::renderer::prelude::renderer::RenderingContext;

pub fn is_pressed(state: ElementState) -> bool {
    match state {
        ElementState::Pressed => true,
        ElementState::Released => false,
    }
}

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

    let mut renderer = NaiveRenderer::new(ctx, window);

    let mut frame_begin = time::Instant::now();
    let mut fps_counter = 0;

    let mut last_frame_time = time::Instant::now();
    let mut dt = 0f32;
    let speed = 1f32;

    let mut forward_pressed = false;
    let mut backward_pressed = false;
    let mut left_pressed = false;
    let mut right_pressed = false;

    let mut yaw = 0f32;
    let mut pitch = 0f32;
    let look_speed = 0.4;

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }

        Event::DeviceEvent {
            event: DeviceEvent::MouseMotion { delta: (x, y) },
            ..
        } => {
            yaw += x as f32 * dt * look_speed;
            pitch += y as f32 * dt * look_speed;
        }

        Event::WindowEvent {
            event:
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state,
                            virtual_keycode,
                            ..
                        },
                    ..
                },
            ..
        } => match virtual_keycode {
            Some(VirtualKeyCode::W) => {
                forward_pressed = is_pressed(state);
            }
            Some(VirtualKeyCode::S) => {
                backward_pressed = is_pressed(state);
            }
            Some(VirtualKeyCode::A) => {
                left_pressed = is_pressed(state);
            }
            Some(VirtualKeyCode::D) => {
                right_pressed = is_pressed(state);
            }
            None | Some(_) => {}
        },

        Event::MainEventsCleared => {
            let now = time::Instant::now();
            dt = (now - last_frame_time).as_secs_f32();
            last_frame_time = now;

            if forward_pressed {
                renderer.position += vec3(0.0, 0.0, dt * speed);
            }
            if backward_pressed {
                renderer.position += vec3(0.0, 0.0, -dt * speed);
            }
            if left_pressed {
                renderer.position += vec3(-dt * speed, 0.0, 0.0);
            }
            if right_pressed {
                renderer.position += vec3(dt * speed, 0.0, 0.0);
            }
            renderer.rotation = Mat4::from_euler_angles(0.0, pitch, yaw);

            renderer.draw();
            fps_counter += 1;

            if now - frame_begin > time::Duration::new(1, 0) {
                frame_begin = now;
                debug!("FPS: {}", fps_counter);
                fps_counter = 0;
            }
        }
        _ => (),
    });
}
