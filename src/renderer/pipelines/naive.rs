extern crate nalgebra_glm as glm;

use std::sync::Arc;

use log::error;
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, ClearColorImageInfo, CommandBufferUsage},
    device::Queue,
    format::ClearColorValue,
    image::{ImageUsage, SwapchainImage},
    swapchain::{self, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo},
    sync::{self, GpuFuture},
};

use crate::{Renderer, RenderingContext};

pub struct NaiveCamera {
    pub position: glm::Vec3,
    pub rotation: glm::Quat,

    // Radians
    pub fov: f32,
}

impl Default for NaiveCamera {
    fn default() -> Self {
        Self {
            position: Default::default(),
            rotation: Default::default(),
            fov: 90f32,
        }
    }
}

pub struct NaiveRenderer {
    pub(crate) ctx: Arc<RenderingContext>,

    // Dataflow
    pub(crate) queue: Arc<Queue>,

    // Presentation
    pub(crate) swapchain: Arc<Swapchain>,
    pub(crate) swapchain_images: Vec<Arc<SwapchainImage>>,
}

impl NaiveRenderer {
    pub fn new(ctx: Arc<RenderingContext>, surface: Arc<Surface>) -> Self {
        let caps = ctx
            .physical_device
            .surface_capabilities(&surface, Default::default())
            .expect("failed to get surface capabilities");

        let dimensions = [800, 600];

        let composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();
        let image_format = Some(
            ctx.physical_device
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0,
        );

        let (swapchain, images) = Swapchain::new(
            ctx.device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: caps.min_image_count + 1, // How many buffers to use in the swapchain
                image_format,
                image_extent: dimensions.into(),
                image_usage: ImageUsage::TRANSFER_DST, // What the images are going to be used for
                composite_alpha,
                ..Default::default()
            },
        )
        .unwrap();

        let queue = ctx.queues.iter().next().unwrap().clone();

        Self {
            ctx,
            swapchain,
            swapchain_images: images,
            queue,
        }
    }

    pub fn draw(&self) {
        let mut builder = AutoCommandBufferBuilder::primary(
            &self.ctx.command_buffer_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let (image_i, _suboptimal, acquire_future) =
            swapchain::acquire_next_image(self.swapchain.clone(), None).unwrap();
        let image = self.swapchain_images.get(image_i as usize).unwrap();

        builder
            .clear_color_image(ClearColorImageInfo {
                clear_value: ClearColorValue::Float([
                    225f32 / 255f32,
                    0f32 / 255f32,
                    152f32 / 255f32,
                    1.0,
                ]),
                ..ClearColorImageInfo::image(image.clone())
            })
            .unwrap();

        let command_buffer = builder.build().unwrap();

        let execution = sync::now(self.ctx.device.clone())
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_i),
            )
            .then_signal_fence_and_flush();

        match execution {
            Ok(future) => {
                future.wait(None).unwrap(); // wait for the GPU to finish
            }
            Err(e) => {
                error!("Failed to flush future: {e}");
            }
        }
    }
}

impl Renderer for NaiveRenderer {
    fn context(&self) -> Arc<RenderingContext> {
        self.ctx.to_owned()
    }
}
