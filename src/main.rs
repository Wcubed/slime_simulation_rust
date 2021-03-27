use crate::system::System;
use imgui::{im_str, Condition, Window};
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBuffer};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::descriptor::pipeline_layout::PipelineLayoutDesc;
use vulkano::descriptor::PipelineLayoutAbstract;
use vulkano::pipeline::ComputePipeline;
use vulkano::sync::GpuFuture;

mod system;

fn main() {
    println!("Hello, world!");

    let system = System::init("Slime Simulation");

    // Compute shader commands are taken from here: https://vulkano.rs/guide/compute-intro
    let data_iter = 0..65536;
    let data_buffer =
        CpuAccessibleBuffer::from_iter(system.device.clone(), BufferUsage::all(), false, data_iter)
            .expect("Failed to create buffer");

    let shader =
        shader::Shader::load(system.device.clone()).expect("Failed to create shader module.");
    let compute_pipeline = Arc::new(
        ComputePipeline::new(system.device.clone(), &shader.main_entry_point(), &(), None)
            .expect("Failed to create compute pipeline"),
    );

    let layout = compute_pipeline.layout().descriptor_set_layout(0).unwrap();
    let set = Arc::new(
        PersistentDescriptorSet::start(layout.clone())
            .add_buffer(data_buffer.clone())
            .unwrap()
            .build()
            .unwrap(),
    );

    let mut builder =
        AutoCommandBufferBuilder::new(system.device.clone(), system.queue.family()).unwrap();
    builder
        .dispatch([1024, 1, 1], compute_pipeline.clone(), set.clone(), ())
        .unwrap();
    let command_buffer = builder.build().unwrap();

    let finished = command_buffer.execute(system.queue.clone()).unwrap();
    finished
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();

    let content = data_buffer.read().unwrap();
    for (n, val) in content.iter().enumerate() {
        println!("{}: {}", n, val);
    }

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

layout(local_size_x = 64, local_size_y = 1, local_size_z = 1) in;

layout(set = 0, binding = 0) buffer Data {
    uint data[];
} buf;

void main() {
    uint idx = gl_GlobalInvocationID.x;
    buf.data[idx] *= 12;
}
"
    }
}
