use crate::system::System;
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

    let mut builder =
        AutoCommandBufferBuilder::new(system.device.clone(), system.queue.family()).unwrap();
    builder
        .dispatch(
            [1024 / 8, 1024 / 8, 1],
            compute_pipeline.clone(),
            set.clone(),
            (),
        )
        .unwrap();
    let command_buffer = builder.build().unwrap();

    let finished = command_buffer.execute(system.queue.clone()).unwrap();
    finished
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();

    // ---- Window imgui loop ----

    system.main_loop(image, move |_, ui| {
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

uint hash(uint state) {
    state ^= 2747636419u;
    state *= 2654435769u;
    state ^= state >> 16;
    state *= 2654435769u;
    state ^= state >> 16;
    state *= 2654435769u;
    return state;
}

void main() {    
    highp uint index = gl_GlobalInvocationID.y * imageSize(img).y + gl_GlobalInvocationID.x;
    float pseudorandom = hash(index) / 4294967295.0;

    vec4 to_write = vec4(vec3(pseudorandom), 1.0);
    imageStore(img, ivec2(gl_GlobalInvocationID.xy), to_write);
}
"
    }
}
