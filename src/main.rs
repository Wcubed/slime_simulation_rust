use crate::system::System;
use imgui::{im_str, Condition, Window};
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::pipeline::ComputePipeline;

mod system;

fn main() {
    println!("Hello, world!");

    let system = System::init("Slime Simulation");

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
