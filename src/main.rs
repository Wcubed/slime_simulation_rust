use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::device::{Device, DeviceExtensions};
use vulkano::image::ImageUsage;
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::swapchain::{
    ColorSpace, FullscreenExclusive, PresentMode, SurfaceTransform, Swapchain,
};
use vulkano_win::VkSurfaceBuild;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

fn main() {
    // Basic commands taken from the vulkano guide: https://vulkano.rs/guide/introduction

    println!("Hello, world!");

    let instance = {
        let extensions = vulkano_win::required_extensions();
        Instance::new(None, &extensions, None).expect("Failed to create instance.")
    };

    let physical = PhysicalDevice::enumerate(&instance)
        .next()
        .expect("No device available");

    // Debug print all the queue families.
    for family in physical.queue_families() {
        println!(
            "Found a queue family with {:?} queues. Supports graphics: {:?}, compute: {:?}",
            family.queues_count(),
            family.supports_graphics(),
            family.supports_compute()
        );
    }

    let queue_family = physical
        .queue_families()
        .find(|&q| q.supports_graphics())
        .expect("Device does not have a queue family that supports graphics.");

    let (device, mut queues) = {
        let device_ext = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::none()
        };

        Device::new(
            physical,
            physical.supported_features(),
            &device_ext,
            [(queue_family, 0.5)].iter().cloned(),
        )
        .expect("Failed to create device")
    };

    let queue = queues.next().unwrap();

    // ---- Setup window ----
    let events_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .build_vk_surface(&events_loop, instance.clone())
        .unwrap();

    // ---- Setup Swapchain ----
    let (swapchain, images) = {
        let caps = surface
            .capabilities(physical)
            .expect("Failed to get capabilities.");
        let dimensions = caps.current_extent.unwrap_or([1280, 1024]);
        let alpha = caps.supported_composite_alpha.iter().next().unwrap();
        let format = caps.supported_formats[0].0;

        Swapchain::new(
            device.clone(),
            surface.clone(),
            caps.min_image_count,
            format,
            dimensions,
            1,
            ImageUsage::color_attachment(),
            &queue,
            SurfaceTransform::Identity,
            alpha,
            PresentMode::Fifo,
            FullscreenExclusive::Default,
            true,
            ColorSpace::SrgbNonLinear,
        )
        .expect("Failed to create swapchain")
    };

    events_loop.run(|event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => (),
        }
    });
}
