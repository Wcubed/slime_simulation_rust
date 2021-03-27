use imgui::{Context, Ui};
use imgui_vulkano_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::sync::Arc;
use std::time::{Duration, Instant};
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::format::Format;
use vulkano::image::{ImageUsage, StorageImage, SwapchainImage};
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::swapchain;
use vulkano::swapchain::{
    AcquireError, ColorSpace, FullscreenExclusive, PresentMode, Surface, SurfaceTransform,
    Swapchain, SwapchainCreationError,
};
use vulkano::sync;
use vulkano::sync::{FlushError, GpuFuture};
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
    pub imgui: Context,
    pub platform: WinitPlatform,
    pub renderer: Renderer,
}

impl System {
    pub fn init(window_title: &str) -> System {
        // Basic commands taken from the vulkano imgui examples:
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
            .with_title(window_title.to_owned())
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
                // Needed for compute shaders.
                khr_storage_buffer_storage_class: true,
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

        let mut format = Format::R8G8B8A8Unorm;

        let (swapchain, images) = {
            let caps = surface
                .capabilities(physical)
                .expect("Failed to get capabilities.");
            format = caps.supported_formats[0].0;
            let dimensions = caps.current_extent.unwrap_or([1280, 1024]);
            let alpha = caps.supported_composite_alpha.iter().next().unwrap();

            let image_usage = ImageUsage {
                transfer_destination: true,
                ..ImageUsage::color_attachment()
            };

            Swapchain::new(
                device.clone(),
                surface.clone(),
                caps.min_image_count,
                format,
                dimensions,
                1,
                image_usage,
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

        let mut imgui = Context::create();
        imgui.set_ini_filename(None);

        let mut platform = WinitPlatform::init(&mut imgui);
        platform.attach_window(imgui.io_mut(), &surface.window(), HiDpiMode::Rounded);

        let renderer = Renderer::init(&mut imgui, device.clone(), queue.clone(), format)
            .expect("Failed to initialize renderer");

        System {
            event_loop,
            device,
            queue,
            surface,
            swapchain,
            images,
            imgui,
            platform,
            renderer,
        }
    }

    pub fn main_loop<F: FnMut(&mut bool, &mut Ui) + 'static>(
        self,
        display_image: Arc<StorageImage<Format>>,
        mut run_ui: F,
    ) {
        let System {
            event_loop,
            device,
            queue,
            surface,
            mut swapchain,
            mut images,
            mut imgui,
            mut platform,
            mut renderer,
            ..
        } = self;

        // Apparently there are various reasons why we might need to re-create the swapchain.
        // For example when the target surface has changed size.
        // This keeps track of whether the previous frame encountered one of those reasons.
        let mut recreate_swapchain = false;
        let mut previous_frame_end = Some(sync::now(device.clone()).boxed());
        let mut last_redraw = Instant::now();

        // target 60 fps
        let target_frame_time = Duration::from_millis(1000 / 60);

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::MainEventsCleared => {
                    platform
                        .prepare_frame(imgui.io_mut(), &surface.window())
                        .expect("Failed to prepare frame.");
                    surface.window().request_redraw();
                }
                Event::RedrawRequested(_) => {
                    // ---- Stick to the framerate ----
                    let t = Instant::now();
                    let since_last = t.duration_since(last_redraw);
                    last_redraw = t;

                    if since_last < target_frame_time {
                        std::thread::sleep(target_frame_time - since_last);
                    }

                    // ---- Cleanup ----

                    previous_frame_end.as_mut().unwrap().cleanup_finished();

                    // ---- Recreate swapchain if necessary ----

                    if recreate_swapchain {
                        let dimensions: [u32; 2] = surface.window().inner_size().into();
                        let (new_swapchain, new_images) =
                            match swapchain.recreate_with_dimensions(dimensions) {
                                Ok(r) => r,
                                Err(SwapchainCreationError::UnsupportedDimensions) => return,
                                Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                            };

                        images = new_images;
                        swapchain = new_swapchain;
                        recreate_swapchain = false;
                    }

                    // ---- Run the user's imgui code ----

                    let mut ui = imgui.frame();
                    let mut run = true;

                    run_ui(&mut run, &mut ui);

                    if !run {
                        *control_flow = ControlFlow::Exit;
                    }

                    // ---- Create draw commands ----

                    let (image_num, suboptimal, acquire_future) =
                        match swapchain::acquire_next_image(swapchain.clone(), None) {
                            Ok(r) => r,
                            Err(AcquireError::OutOfDate) => {
                                recreate_swapchain = true;
                                return;
                            }
                            Err(e) => panic!("Failed to acquire next image: {:?}", e),
                        };

                    if suboptimal {
                        recreate_swapchain = true;
                    }

                    platform.prepare_render(&ui, surface.window());
                    let draw_data = ui.render();

                    let extent_x = display_image
                        .dimensions()
                        .width()
                        .min(images[image_num].dimensions()[0]);
                    let extent_y = display_image
                        .dimensions()
                        .height()
                        .min(images[image_num].dimensions()[1]);

                    let mut cmd_buf_builder =
                        AutoCommandBufferBuilder::new(device.clone(), queue.family())
                            .expect("Failed to create command buffer");
                    // Clear screen and show the desired image.
                    cmd_buf_builder
                        .clear_color_image(images[image_num].clone(), [0.0; 4].into())
                        .unwrap()
                        .copy_image(
                            display_image.clone(),
                            [0; 3],
                            0,
                            0,
                            images[image_num].clone(),
                            [0; 3],
                            0,
                            0,
                            [extent_x, extent_y, 1],
                            1,
                        )
                        .expect("Failed to create image copy command");

                    renderer
                        .draw_commands(
                            &mut cmd_buf_builder,
                            queue.clone(),
                            images[image_num].clone(),
                            draw_data,
                        )
                        .expect("Rendering failed");

                    let cmd_buf = cmd_buf_builder
                        .build()
                        .expect("Failed to build command buffer");

                    // ---- Execute the draw commands ----

                    let future = previous_frame_end
                        .take()
                        .unwrap()
                        .join(acquire_future)
                        .then_execute(queue.clone(), cmd_buf)
                        .unwrap()
                        .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
                        .then_signal_fence_and_flush();

                    match future {
                        Ok(future) => {
                            previous_frame_end = Some(future.boxed());
                        }
                        Err(FlushError::OutOfDate) => {
                            recreate_swapchain = true;
                            previous_frame_end = Some(sync::now(device.clone()).boxed());
                        }
                        Err(e) => {
                            println!("Failed to flush future: {:?}", e);
                            previous_frame_end = Some(sync::now(device.clone()).boxed());
                        }
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                }
                event => {
                    // Pass events on to imgui.
                    platform.handle_event(imgui.io_mut(), surface.window(), &event);
                }
            }
        });
    }
}
