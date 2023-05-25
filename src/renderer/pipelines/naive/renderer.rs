extern crate nalgebra_glm as glm;

use std::sync::Arc;

use crate::{naive::constants::RendererConstants, Camera, RawCamera};
use glm::{vec3, Vec3};
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage},
    command_buffer::{AutoCommandBufferBuilder, BlitImageInfo, CommandBufferUsage},
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator,
        layout::{
            DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo,
            DescriptorType,
        },
        PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::Queue,
    image::{view::ImageView, ImageDimensions, ImageUsage, StorageImage, SwapchainImage},
    memory::allocator::{AllocationCreateInfo, MemoryUsage},
    pipeline::{
        layout::{PipelineLayoutCreateInfo, PushConstantRange},
        ComputePipeline, Pipeline, PipelineBindPoint, PipelineLayout,
    },
    shader::ShaderStages,
    swapchain::{self, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo},
    sync::{self, GpuFuture},
};

use crate::RenderingContext;

use super::{shader, Sphere};

pub struct NaiveRenderer {
    pub(crate) ctx: Arc<RenderingContext>,

    pub(crate) surface_size: [u32; 2],
    pub(crate) viewport_size: [u32; 2],
    pub(crate) scale_factor: u32,

    // Dataflow
    pub(crate) queue: Arc<Queue>,
    pub(crate) out_image: Arc<StorageImage>,
    pub(crate) pipeline: Arc<ComputePipeline>,
    pub(crate) descriptors: Arc<PersistentDescriptorSet>,

    // Presentation
    pub(crate) swapchain: Arc<Swapchain>,
    pub(crate) swapchain_images: Vec<Arc<SwapchainImage>>,

    // Controls
    pub(crate) position: Vec3,
    pub(crate) rotation: Vec3,
}

impl NaiveRenderer {
    pub fn new(ctx: Arc<RenderingContext>, surface: Arc<Surface>) -> Self {
        // Capabilities of the surface of the device
        let caps = ctx
            .physical_device
            .surface_capabilities(&surface, Default::default())
            .expect("failed to get surface capabilities");

        // Dimensions of the surface to draw on
        let surface_size = [800, 600];
        let scale_factor = 1u32;
        let viewport_size = [800 / scale_factor, 600 / scale_factor];

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
                image_extent: surface_size.into(),
                image_usage: ImageUsage::TRANSFER_DST, // What the images are going to be used for
                composite_alpha,
                ..Default::default()
            },
        )
        .unwrap();

        // Queue to push the commands into
        let queue = ctx.queues.iter().next().unwrap().clone();

        // The constants set up by the renderer
        let consts = RendererConstants {
            aspect_ratio: viewport_size[0] as f32 / viewport_size[1] as f32,
            width: viewport_size[0],
            height: viewport_size[1],
            min_depth: 0f32,
            max_depth: 12f32,
        };

        // The buffer to draw onto
        let out_image = StorageImage::new(
            &ctx.memory_allocator,
            ImageDimensions::Dim2d {
                width: viewport_size[0],
                height: viewport_size[1],
                array_layers: 1,
            },
            vulkano::format::Format::R8G8B8A8_UNORM,
            Some(queue.queue_family_index()),
        )
        .unwrap();

        // The buffer to store spheres in
        let sphere_buffer = Buffer::from_iter(
            &ctx.memory_allocator,
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                usage: MemoryUsage::Upload,
                ..Default::default()
            },
            [
                Sphere::new(vec3(1.0, 0.0, -1.0), 0.2),
                Sphere::new(vec3(0.0, 0.0, -1.0), 0.5),
            ]
            .map(Sphere::raw),
        )
        .unwrap();

        // Allocator for the descriptors
        let descriptor_set_allocator = StandardDescriptorSetAllocator::new(ctx.device.clone());

        // Layout of the descriptors in the set
        let descriptor_set_layout = DescriptorSetLayout::new(
            ctx.device.clone(),
            DescriptorSetLayoutCreateInfo {
                bindings: [
                    (
                        0,
                        DescriptorSetLayoutBinding {
                            stages: ShaderStages::COMPUTE,
                            ..DescriptorSetLayoutBinding::descriptor_type(
                                DescriptorType::StorageImage,
                            )
                        },
                    ),
                    (
                        1,
                        DescriptorSetLayoutBinding {
                            stages: ShaderStages::COMPUTE,
                            ..DescriptorSetLayoutBinding::descriptor_type(
                                DescriptorType::UniformBuffer,
                            )
                        },
                    ),
                    (
                        2,
                        DescriptorSetLayoutBinding {
                            stages: ShaderStages::COMPUTE,
                            ..DescriptorSetLayoutBinding::descriptor_type(
                                DescriptorType::UniformBuffer,
                            )
                        },
                    ),
                ]
                .into(),
                ..Default::default()
            },
        )
        .unwrap();

        // Compiled shader to display colour with
        let shader = shader(ctx.device.clone());

        // Push constants
        let push_constants = vec![PushConstantRange {
            stages: ShaderStages::COMPUTE,
            size: std::mem::size_of::<RawCamera>() as u32,
            ..Default::default()
        }];

        // The set of inputs that a pipeline processes
        let pipeline_layout = PipelineLayout::new(
            ctx.device.clone(),
            PipelineLayoutCreateInfo {
                set_layouts: vec![descriptor_set_layout.clone()],
                push_constant_ranges: push_constants,
                ..Default::default()
            },
        )
        .unwrap();

        // The single shader compute pipeline to run the operations inside of
        let compute_pipeline = ComputePipeline::with_pipeline_layout(
            ctx.device.clone(),
            shader.entry_point("main").unwrap(),
            &consts,
            pipeline_layout,
            None,
        )
        .expect("failed to create compute pipeline");

        // View of the image for the pipeline to draw on
        let view = ImageView::new_default(out_image.clone()).unwrap();

        // Descriptors to push into the pipeline
        let descriptor_set = PersistentDescriptorSet::new(
            &descriptor_set_allocator,
            descriptor_set_layout.clone(),
            [
                WriteDescriptorSet::image_view(0, view),
                WriteDescriptorSet::buffer(1, sphere_buffer),
            ],
        )
        .unwrap();

        Self {
            position: Vec3::identity(),
            rotation: Vec3::identity(),
            ctx,
            scale_factor,
            viewport_size,
            surface_size,
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

        builder
            .bind_pipeline_compute(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Compute,
                self.pipeline.layout().clone(),
                0u32,
                self.descriptors.clone(),
            )
            .push_constants(
                self.pipeline.layout().clone(),
                0,
                Camera {
                    position: self.position,
                    rotation: self.rotation,
                }
                .raw(),
            )
            .dispatch([self.viewport_size[0], self.viewport_size[1], 1])
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
