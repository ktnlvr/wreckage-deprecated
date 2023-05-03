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

        let mut physical_devices = vulkan_instance
            .enumerate_physical_devices()
            .expect("could not enumerate devices")
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .collect::<Vec<_>>();

        physical_devices.sort_unstable_by_key(|device| match device.properties().device_type {
            vulkano::device::physical::PhysicalDeviceType::DiscreteGpu => 0,
            vulkano::device::physical::PhysicalDeviceType::IntegratedGpu => 1,
            vulkano::device::physical::PhysicalDeviceType::VirtualGpu => 2,
            vulkano::device::physical::PhysicalDeviceType::Cpu => 3,
            vulkano::device::physical::PhysicalDeviceType::Other => 4,
            _ => 5,
        });

        info!("Found {} devices:", physical_devices.len());
        for device in &physical_devices {
            info!(
                "  {} ({:?})",
                device.properties().device_name,
                device.properties().device_type
            );
        }

        let physical_device = physical_devices[0].clone();
        info!(
            "Selected {} (driver v{})",
            physical_device.properties().device_name,
            physical_device
                .properties()
                .driver_info
                .to_owned()
                .unwrap_or("`undefined`".into())
        );

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
            physical_device: physical_device.clone(),
            device,
            queues,
        }))
    }
}

pub trait Renderer: Send {
    fn context(&self) -> Arc<RenderingContext>;
}
