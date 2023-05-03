extern crate nalgebra_glm as glm;

use std::sync::Arc;

use glm::vec3;
use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
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
    image::{
        view::ImageView, ImageAccess, ImageDimensions, ImageUsage, StorageImage, SwapchainImage,
    },
    memory::allocator::{AllocationCreateInfo, MemoryUsage},
    pipeline::{
        layout::{PipelineLayoutCreateInfo},
        ComputePipeline, Pipeline, PipelineBindPoint, PipelineLayout,
    },
    shader::{DescriptorBindingRequirements, ShaderStages},
    swapchain::{self, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo},
    sync::{self, GpuFuture},
};

use crate::RenderingContext;

use super::{shader, RawSphere, Sphere};

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
    pub(crate) min_depth: f32,
    pub(crate) max_depth: f32,
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

        let consts = RendererConstants {
            aspect_ratio: 800f32 / 600f32,
            width: 800f32,
            height: 600f32,
            min_depth: 0f32,
            max_depth: 40f32,
        };

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
                Sphere::new(vec3(3.0, 1.0, -4.0), 0.2),
                Sphere::new(vec3(0.0, 0.0, -1.0), 0.5),
            ]
            .map(Sphere::raw),
        )
        .unwrap();

        let render_constants = Buffer::from_data(
            &ctx.memory_allocator,
            BufferCreateInfo {
                usage: BufferUsage::UNIFORM_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                usage: MemoryUsage::Upload,
                ..Default::default()
            },
            consts,
        ).unwrap();

        let descriptor_set_allocator = StandardDescriptorSetAllocator::new(ctx.device.clone());
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

        let pipeline_layout = PipelineLayout::new(
            ctx.device.clone(),
            PipelineLayoutCreateInfo {
                set_layouts: vec![descriptor_set_layout.clone()],
                push_constant_ranges: vec![],
                ..Default::default()
            },
        )
        .unwrap();

        let compute_pipeline = ComputePipeline::with_pipeline_layout(
            ctx.device.clone(),
            shader(ctx.device.clone()).entry_point("main").unwrap(),
            &(),
            pipeline_layout,
            None,
        )
        .expect("failed to create compute pipeline");

        let view = ImageView::new_default(out_image.clone()).unwrap();

        let descriptor_set = PersistentDescriptorSet::new(
            &descriptor_set_allocator,
            descriptor_set_layout.clone(),
            [
                WriteDescriptorSet::image_view(0, view),
                WriteDescriptorSet::buffer(1, render_constants),
                WriteDescriptorSet::buffer(2, sphere_buffer),
            ],
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

        builder
            .bind_pipeline_compute(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Compute,
                self.pipeline.layout().clone(),
                0u32,
                self.descriptors.clone(),
            )
            .dispatch([800, 600, 2])
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
