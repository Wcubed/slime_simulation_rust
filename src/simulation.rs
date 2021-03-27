use std::sync::Arc;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBuffer};
use vulkano::descriptor::descriptor_set::{PersistentDescriptorSet, PersistentDescriptorSetImg};
use vulkano::descriptor::pipeline_layout::PipelineLayout;
use vulkano::descriptor::PipelineLayoutAbstract;
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::image::{Dimensions, StorageImage};
use vulkano::pipeline::ComputePipeline;
use vulkano::sync::GpuFuture;

pub struct Simulation {
    pub image: Arc<StorageImage<Format>>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub pipeline: Arc<ComputePipeline<PipelineLayout<shader::Layout>>>,
    pub set: Arc<
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

        let shader = shader::Shader::load(device.clone()).expect("failed to create shader module");

        let pipeline = Arc::new(
            ComputePipeline::new(device.clone(), &shader.main_entry_point(), &(), None)
                .expect("failed to create compute pipeline"),
        );

        let set = Arc::new(
            PersistentDescriptorSet::start(
                pipeline.layout().descriptor_set_layout(0).unwrap().clone(),
            )
            .add_image(image.clone())
            .unwrap()
            .build()
            .unwrap(),
        );

        Simulation {
            image,
            device,
            queue,
            pipeline,
            set,
        }
    }

    pub fn run_once(&self) {
        let mut builder =
            AutoCommandBufferBuilder::new(self.device.clone(), self.queue.family()).unwrap();
        builder
            .dispatch(
                [1024 / 8, 1024 / 8, 1],
                self.pipeline.clone(),
                self.set.clone(),
                (),
            )
            .unwrap();
        let command_buffer = builder.build().unwrap();

        let finished = command_buffer.execute(self.queue.clone()).unwrap();
        finished
            .then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();
    }
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
    float r = hash(index) / 4294967295.0;
    float g = hash(index + 1000000) / 4294967295.0;
    float b = hash(index + 696000000) / 4294967295.0;

    vec4 to_write = vec4(r, g, b, 1.0);
    imageStore(img, ivec2(gl_GlobalInvocationID.xy), to_write);
}
"
    }
}
