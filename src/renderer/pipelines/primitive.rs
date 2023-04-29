use std::{error::Error, sync::Arc};

use log::info;
use vulkano::device::{DeviceCreateInfo, DeviceExtensions, QueueCreateInfo, QueueFlags};
use vulkano::{
    device::{Device, Queue},
    instance::{Instance, InstanceCreateInfo, InstanceExtensions},
    memory::allocator::StandardMemoryAllocator,
    VulkanLibrary,
};

use crate::Renderer;

pub struct PrimitiveRenderer {
    pub(crate) instance: Arc<Instance>,

    pub(crate) allocator: StandardMemoryAllocator,
    pub(crate) device: Arc<Device>,
    pub(crate) queue: Arc<Queue>,
}

impl PrimitiveRenderer {
    pub fn new(
        vulkan_library: Arc<VulkanLibrary>,
        required_extensions: InstanceExtensions,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let required_instance_extensions = InstanceExtensions {
            ..required_extensions
        };

        let vulkan_instance = Instance::new(
            vulkan_library.to_owned(),
            InstanceCreateInfo {
                application_name: Some("Wreckage".into()),
                enabled_extensions: required_instance_extensions,
                ..Default::default()
            },
        )?;

        let physical_device = vulkan_instance
            .enumerate_physical_devices()
            .expect("could not enumerate devices")
            .next()
            .expect("no devices available");

        for family in physical_device.queue_family_properties() {
            info!(
                "Found a queue ({:?}) family with {:?} queue(s)",
                family.queue_flags, family.queue_count
            );
        }

        let queue_family_index = physical_device
            .queue_family_properties()
            .iter()
            .enumerate()
            .position(|(_queue_family_index, queue_family_properties)| {
                queue_family_properties
                    .queue_flags
                    .contains(QueueFlags::GRAPHICS | QueueFlags::TRANSFER | QueueFlags::COMPUTE)
            })
            .expect("couldn't find a graphical queue family")
            as u32;

        let required_device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..Default::default()
        };

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: required_device_extensions,
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        )
        .expect("failed to create device");

        let queue = queues.next().unwrap();
        let allocator = StandardMemoryAllocator::new_default(device.clone());

        Ok(Self {
            instance: vulkan_instance,
            allocator,
            device,
            queue,
        })
    }
}

impl Renderer for PrimitiveRenderer {
    fn instance(&self) -> Arc<Instance> {
        self.instance.clone()
    }

    fn render_all(&mut self) {}
}
