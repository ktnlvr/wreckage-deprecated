extern crate nalgebra_glm as glm;

use std::sync::Arc;

use vulkano::{
    buffer::BufferContents,
    command_buffer::{AutoCommandBufferBuilder, BlitImageInfo, CommandBufferUsage},
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator, PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::Queue,
    image::{
        view::ImageView, ImageAccess, ImageDimensions, ImageUsage, StorageImage, SwapchainImage,
    },
    pipeline::{layout::PushConstantRange, ComputePipeline, Pipeline, PipelineBindPoint},
    swapchain::{self, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo},
    sync::{self, GpuFuture},
};

use crate::RenderingContext;

use super::shader;

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
    pub(crate) out_image: Arc<StorageImage>,
    pub(crate) pipeline: Arc<ComputePipeline>,
    pub(crate) descriptors: Arc<PersistentDescriptorSet>,

    // Presentation
    pub(crate) swapchain: Arc<Swapchain>,
    pub(crate) swapchain_images: Vec<Arc<SwapchainImage>>,
}

#[derive(BufferContents)]
#[repr(C)]
pub struct RendererConstants {
    pub(crate) aspect_ratio: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

impl NaiveRenderer {
    pub fn new(ctx: Arc<RenderingContext>, surface: Arc<Surface>) -> Self {
        let caps = ctx
            .physical_device
            .surface_capabilities(&surface, Default::default())
            .expect("failed to get surface capabilities");

        // Dimensions of the surface to draw on
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

        // The buffer to draw onto
        let out_image = StorageImage::new(
            &ctx.memory_allocator,
            ImageDimensions::Dim2d {
                width: dimensions[0],
                height: dimensions[1],
                array_layers: 1, // images can be arrays of layers
            },
            vulkano::format::Format::R8G8B8A8_UNORM,
            Some(queue.queue_family_index()),
        )
        .unwrap();

        let compute_pipeline = ComputePipeline::new(
            ctx.device.clone(),
            shader(ctx.device.clone()).entry_point("main").unwrap(),
            &(),
            None,
            |_| {},
        )
        .expect("failed to create compute pipeline");

        let descriptor_set_allocator = StandardDescriptorSetAllocator::new(ctx.device.clone());
        let pipeline_layout = compute_pipeline.layout();
        let descriptor_set_layouts = pipeline_layout.set_layouts();

        let descriptor_set_layout_index = 0;
        let descriptor_set_layout = descriptor_set_layouts
            .get(descriptor_set_layout_index)
            .unwrap();

        let view = ImageView::new_default(out_image.clone()).unwrap();

        let descriptor_set = PersistentDescriptorSet::new(
            &descriptor_set_allocator,
            descriptor_set_layout.clone(),
            [WriteDescriptorSet::image_view(0, view)], // 0 is the binding
        )
        .unwrap();

        Self {
            ctx,
            swapchain,
            swapchain_images: images,
            out_image,
            pipeline: compute_pipeline,
            descriptors: descriptor_set,
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

        let camera = NaiveCamera::default();

        let consts = RendererConstants {
            aspect_ratio: image.dimensions().width() as f32 / image.dimensions().height() as f32,
            width: image.dimensions().width() as f32,
            height: image.dimensions().height() as f32,
        };

        builder
            .bind_pipeline_compute(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Compute,
                self.pipeline.layout().clone(),
                0u32,
                self.descriptors.clone(),
            )
            .push_constants(self.pipeline.layout().clone(), 0, consts)
            .dispatch([800, 600, 1])
            .unwrap()
            .blit_image(BlitImageInfo::images(self.out_image.clone(), image.clone()))
            .unwrap();

        let command_buffer = builder.build().unwrap();

        sync::now(self.ctx.device.clone())
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_i),
            )
            .then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();
    }
}
