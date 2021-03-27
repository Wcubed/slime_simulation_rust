use crate::system::System;
use image::{ImageBuffer, Rgba};
use imgui::{im_str, Condition, Window};
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBuffer};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::descriptor::pipeline_layout::PipelineLayoutDesc;
use vulkano::descriptor::PipelineLayoutAbstract;
use vulkano::format::{ClearValue, Format};
use vulkano::image::{Dimensions, StorageImage};
use vulkano::pipeline::ComputePipeline;
use vulkano::sync::GpuFuture;

mod system;

fn main() {
    println!("Hello, world!");

    let system = System::init("Slime Simulation");

    // ---- Computing to an image buffer ----

    let image = StorageImage::new(
        system.device.clone(),
        Dimensions::Dim2d {
            width: 1024,
            height: 1024,
        },
        Format::R8G8B8A8Unorm,
        Some(system.queue.family()),
    )
    .unwrap();

    let shader =
        shader::Shader::load(system.device.clone()).expect("failed to create shader module");

    let compute_pipeline = Arc::new(
        ComputePipeline::new(system.device.clone(), &shader.main_entry_point(), &(), None)
            .expect("failed to create compute pipeline"),
    );

    let set = Arc::new(
        PersistentDescriptorSet::start(
            compute_pipeline
                .layout()
                .descriptor_set_layout(0)
                .unwrap()
                .clone(),
        )
        .add_image(image.clone())
        .unwrap()
        .build()
        .unwrap(),
    );

    let buf = CpuAccessibleBuffer::from_iter(
        system.device.clone(),
        BufferUsage::all(),
        false,
        (0..1024 * 1024 * 4).map(|_| 0u8),
    )
    .expect("failed to create buffer");

    let mut builder =
        AutoCommandBufferBuilder::new(system.device.clone(), system.queue.family()).unwrap();
    builder
        .dispatch(
            [1024 / 8, 1024 / 8, 1],
            compute_pipeline.clone(),
            set.clone(),
            (),
        )
        .unwrap()
        .copy_image_to_buffer(image.clone(), buf.clone())
        .unwrap();
    let command_buffer = builder.build().unwrap();

    let finished = command_buffer.execute(system.queue.clone()).unwrap();
    finished
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();

    let buffer_content = buf.read().unwrap();
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(1024, 1024, &buffer_content[..]).unwrap();
    image.save("image.png").unwrap();

    // ---- Window imgui loop ----

    system.main_loop(move |_, ui| {
        Window::new(im_str!("Hello World!"))
            .size([300.0, 110.0], Condition::FirstUseEver)
            .build(ui, || {
                ui.text(im_str!("Hello World!"));
                ui.button(&im_str!("A button"), [100.0, 30.0]);
            });
    })
}

mod shader {
    vulkano_shaders::shader! {
        ty: "compute",
        src:
"
#version 450

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8) uniform writeonly image2D img;

void main() {
    vec2 norm_coordinates = (gl_GlobalInvocationID.xy + vec2(0.5)) / vec2(imageSize(img));
    vec2 c = (norm_coordinates - vec2(0.5)) * 2.0 - vec2(1.0, 0.0);

    vec2 z = vec2(0.0, 0.0);
    float i;
    for (i = 0.0; i < 1.0; i += 0.005) {
        z = vec2(
        z.x * z.x - z.y * z.y + c.x,
        z.y * z.x + z.x * z.y + c.y
        );

        if (length(z) > 4.0) {
            break;
        }
    }

    vec4 to_write = vec4(vec3(i), 1.0);
    imageStore(img, ivec2(gl_GlobalInvocationID.xy), to_write);
}
"
    }
}
