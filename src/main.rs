use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::device::{Device, DeviceExtensions, Features};
use vulkano::instance::{Instance, InstanceExtensions, PhysicalDevice};
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
        Device::new(
            physical,
            &Features::none(),
            &DeviceExtensions::none(),
            [(queue_family, 0.5)].iter().cloned(),
        )
        .expect("Failed to create device")
    };

    let queue = queues.next().unwrap();

    let data = 12;
    let buffer = CpuAccessibleBuffer::from_data(device.clone(), BufferUsage::all(), false, data)
        .expect("Failed to create buffer");

    let events_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .build_vk_surface(&events_loop, instance.clone())
        .unwrap();

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
