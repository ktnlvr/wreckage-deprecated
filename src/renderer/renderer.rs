use std::{error::Error, sync::Arc};

use log::info;
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::{DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags};
use vulkano::{
    device::Device,
    instance::{Instance, InstanceCreateInfo, InstanceExtensions},
    memory::allocator::StandardMemoryAllocator,
    VulkanLibrary,
};

pub struct RenderingContext {
    pub instance: Arc<Instance>,
    pub memory_allocator: StandardMemoryAllocator,
    pub command_buffer_allocator: StandardCommandBufferAllocator,
    pub physical_device: Arc<PhysicalDevice>,
    pub device: Arc<Device>,
    pub queues: Vec<Arc<Queue>>,
}

impl RenderingContext {
    pub fn new(
        vulkan_library: Arc<VulkanLibrary>,
        instance_extensions: InstanceExtensions,
        device_extensions: DeviceExtensions,
    ) -> Result<Arc<Self>, Box<dyn Error + Send + Sync>> {
        let required_instance_extensions = InstanceExtensions {
            ..instance_extensions
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
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .next()
            .expect("no devices available");

        info!(
            "{} (driver v. {}) features: {}",
            physical_device.properties().device_name,
            physical_device
                .properties()
                .driver_info
                .to_owned()
                .unwrap_or("undefined".into()),
            physical_device.supported_features().into_iter().fold(
                String::new(),
                |mut acc, (feature, enabled)| {
                    if enabled {
                        acc.push_str(feature);
                        acc.push_str(", ");
                    }
                    acc
                }
            )
        );

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

        let (device, queues) = Device::new(
            physical_device.clone(),
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
        let queues = queues.collect();

        let allocator = StandardMemoryAllocator::new_default(device.clone());
        let command_allocator =
            StandardCommandBufferAllocator::new(device.clone(), Default::default());

        Ok(Arc::new(Self {
            instance: vulkan_instance,
            memory_allocator: allocator,
            command_buffer_allocator: command_allocator,
            physical_device,
            device,
            queues,
        }))
    }
}

pub trait Renderer: Send {
    fn context(&self) -> Arc<RenderingContext>;
}
