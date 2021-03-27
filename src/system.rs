use std::sync::Arc;
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::image::{ImageUsage, SwapchainImage};
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::swapchain::{
    ColorSpace, FullscreenExclusive, PresentMode, Surface, SurfaceTransform, Swapchain,
};
use vulkano_win::VkSurfaceBuild;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

pub struct System {
    pub event_loop: EventLoop<()>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub surface: Arc<Surface<Window>>,
    pub swapchain: Arc<Swapchain<Window>>,
    pub images: Vec<Arc<SwapchainImage<Window>>>,
}

impl System {
    pub fn init() -> System {
        // Basic commands taken from the vulkano guide: https://vulkano.rs/guide/introduction
        // and the vulkano imgui examples, here:
        // https://github.com/Tenebryo/imgui-vulkano-renderer/blob/master/examples/support/mod.rs

        let instance = {
            let extensions = vulkano_win::required_extensions();
            Instance::new(None, &extensions, None).expect("Failed to create instance.")
        };

        let physical = PhysicalDevice::enumerate(&instance)
            .next()
            .expect("No device available");

        let event_loop = EventLoop::new();
        let surface = WindowBuilder::new()
            .build_vk_surface(&event_loop, instance.clone())
            .unwrap();

        let queue_family = physical
            .queue_families()
            .find(|&q|
                q.supports_graphics() && q.explicitly_supports_transfers()
                && surface.is_supported(q).unwrap_or(false)
            )
            .expect("Device does not have a queue family that can draw to the window and supports transfers.");

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

        System {
            event_loop,
            device,
            queue,
            surface,
            swapchain,
            images,
        }
    }

    pub fn main_loop(self) {
        let System {
            event_loop,
            device,
            queue,
            surface,
            mut swapchain,
            mut images,
            ..
        } = self;

        event_loop.run(|event, _, control_flow| {
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
}
