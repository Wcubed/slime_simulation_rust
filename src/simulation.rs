use std::sync::Arc;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::descriptor::descriptor_set::{PersistentDescriptorSet, PersistentDescriptorSetImg};
use vulkano::descriptor::pipeline_layout::PipelineLayout;
use vulkano::descriptor::PipelineLayoutAbstract;
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::image::{Dimensions, StorageImage};
use vulkano::pipeline::ComputePipeline;

pub struct Simulation {
    pub result_image: Arc<StorageImage<Format>>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    noise_pipeline: Arc<ComputePipeline<PipelineLayout<noise_shader::Layout>>>,
    noise_set: Arc<
        PersistentDescriptorSet<(
            (),
            PersistentDescriptorSetImg<Arc<vulkano::image::StorageImage<vulkano::format::Format>>>,
        )>,
    >,
    blur_pipeline: Arc<ComputePipeline<PipelineLayout<blur_shader::Layout>>>,
    blur_set: Arc<
        PersistentDescriptorSet<(
            (),
            PersistentDescriptorSetImg<Arc<vulkano::image::StorageImage<vulkano::format::Format>>>,
        )>,
    >,
}

impl Simulation {
    pub fn init(device: Arc<Device>, queue: Arc<Queue>) -> Simulation {
        let image = StorageImage::new(
            device.clone(),
            Dimensions::Dim2d {
                width: 1024,
                height: 1024,
            },
            Format::R8G8B8A8Unorm,
            Some(queue.family()),
        )
        .unwrap();

        let noise_shader =
            noise_shader::Shader::load(device.clone()).expect("failed to create shader module");

        let noise_pipeline = Arc::new(
            ComputePipeline::new(device.clone(), &noise_shader.main_entry_point(), &(), None)
                .expect("failed to create compute pipeline"),
        );

        let noise_set = Arc::new(
            PersistentDescriptorSet::start(
                noise_pipeline
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

        let blur_shader =
            blur_shader::Shader::load(device.clone()).expect("failed to create shader module");

        let blur_pipeline = Arc::new(
            ComputePipeline::new(device.clone(), &blur_shader.main_entry_point(), &(), None)
                .expect("failed to create compute pipeline"),
        );

        let blur_set = Arc::new(
            PersistentDescriptorSet::start(
                blur_pipeline
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

        Simulation {
            result_image: image,
            device,
            queue,
            noise_pipeline,
            noise_set,
            blur_pipeline,
            blur_set,
        }
    }

    /// After running the built commands, the result_image will be filled.
    pub fn build_commands(&self, counter: u32, builder: &mut AutoCommandBufferBuilder) {
        builder
            .dispatch(
                [1024 / 8, 1024 / 8, 1],
                self.noise_pipeline.clone(),
                self.noise_set.clone(),
                noise_shader::ty::PushConstantData { offset: counter },
            )
            .unwrap()
            .dispatch(
                [1024 / 8, 1024 / 8, 1],
                self.blur_pipeline.clone(),
                self.blur_set.clone(),
                (),
            )
            .unwrap();
    }
}

pub mod noise_shader {
    vulkano_shaders::shader! {
        ty: "compute",
        src:
"
#version 450

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8) uniform writeonly image2D img;

layout(push_constant) uniform PushConstantData {
    uint offset;
} pc;

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
    highp uint index = gl_GlobalInvocationID.y * imageSize(img).y + gl_GlobalInvocationID.x + (pc.offset * 1000000);
    float r = hash(index) / 4294967295.0;
    float g = hash(index + 1000000) / 4294967295.0;
    float b = hash(index + 696000000) / 4294967295.0;

    vec4 to_write = vec4(r, g, b, 1.0);
    imageStore(img, ivec2(gl_GlobalInvocationID.xy), to_write);
}
"
    }
}

pub mod blur_shader {
    vulkano_shaders::shader! {
            ty: "compute",
            src:
"
#version 450

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8) uniform writeonly image2D in_img;

void main() {
    
}
"
    }
}
