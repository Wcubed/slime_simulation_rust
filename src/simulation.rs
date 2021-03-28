use rand::Rng;
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::descriptor::descriptor_set::{
    PersistentDescriptorSet, PersistentDescriptorSetBuf, PersistentDescriptorSetImg,
};
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
    agent_sim_pipeline: Arc<ComputePipeline<PipelineLayout<agent_shader::Layout>>>,
    agent_sim_set: Arc<
        PersistentDescriptorSet<(
            (
                (
                    (),
                    PersistentDescriptorSetImg<
                        Arc<vulkano::image::StorageImage<vulkano::format::Format>>,
                    >,
                ),
                PersistentDescriptorSetImg<
                    Arc<vulkano::image::StorageImage<vulkano::format::Format>>,
                >,
            ),
            PersistentDescriptorSetBuf<Arc<CpuAccessibleBuffer<[agent_shader::ty::Agent]>>>,
        )>,
    >,
    agent_sim_image: Arc<StorageImage<Format>>,
    blur_pipeline: Arc<ComputePipeline<PipelineLayout<blur_fade_shader::Layout>>>,
    blur_set: Arc<
        PersistentDescriptorSet<(
            (
                (),
                PersistentDescriptorSetImg<
                    Arc<vulkano::image::StorageImage<vulkano::format::Format>>,
                >,
            ),
            PersistentDescriptorSetImg<Arc<vulkano::image::StorageImage<vulkano::format::Format>>>,
        )>,
    >,
}

impl Simulation {
    pub fn init(device: Arc<Device>, queue: Arc<Queue>) -> Simulation {
        let image_size = Dimensions::Dim2d {
            width: 1024,
            height: 1024,
        };
        let image_format = Format::R8G8B8A8Unorm;

        let agent_sim_image = StorageImage::new(
            device.clone(),
            image_size,
            image_format,
            Some(queue.family()),
        )
        .unwrap();
        let result_image = StorageImage::new(
            device.clone(),
            image_size,
            image_format,
            Some(queue.family()),
        )
        .unwrap();

        let mut rng = rand::thread_rng();

        let agent_iter = (0..100).map(|_i| agent_shader::ty::Agent {
            // No clue what the dummy is for.
            _dummy0: [0u8; 4],
            pos: [
                rng.gen_range(0..image_size.width()) as f32,
                rng.gen_range(0..image_size.height()) as f32,
            ],
            angle: 0.0,
        });
        let agents_buffer =
            CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, agent_iter)
                .unwrap();

        let noise_shader =
            agent_shader::Shader::load(device.clone()).expect("failed to create shader module");

        let agent_sim_pipeline = Arc::new(
            ComputePipeline::new(device.clone(), &noise_shader.main_entry_point(), &(), None)
                .expect("failed to create compute pipeline"),
        );

        let agent_sim_set = Arc::new(
            PersistentDescriptorSet::start(
                agent_sim_pipeline
                    .layout()
                    .descriptor_set_layout(0)
                    .unwrap()
                    .clone(),
            )
            .add_image(result_image.clone())
            .unwrap()
            .add_image(agent_sim_image.clone())
            .unwrap()
            .add_buffer(agents_buffer)
            .unwrap()
            .build()
            .unwrap(),
        );

        let blur_shader =
            blur_fade_shader::Shader::load(device.clone()).expect("failed to create shader module");

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
            .add_image(agent_sim_image.clone())
            .unwrap()
            .add_image(result_image.clone())
            .unwrap()
            .build()
            .unwrap(),
        );

        Simulation {
            result_image,
            device,
            queue,
            agent_sim_pipeline,
            agent_sim_set,
            agent_sim_image,
            blur_pipeline,
            blur_set,
        }
    }

    /// After running the built commands, the result_image will be filled.
    pub fn build_commands(&self, builder: &mut AutoCommandBufferBuilder) {
        builder
            // Transfer old trails.
            .copy_image(
                self.result_image.clone(),
                [0; 3],
                0,
                0,
                self.agent_sim_image.clone(),
                [0; 3],
                0,
                0,
                [
                    self.result_image.dimensions().width(),
                    self.result_image.dimensions().height(),
                    1,
                ],
                1,
            )
            .unwrap()
            .dispatch(
                [100, 1, 1],
                self.agent_sim_pipeline.clone(),
                self.agent_sim_set.clone(),
                agent_shader::ty::PushConstantData {
                    agent_speed: 5.0,
                    delta_time: 0.1,
                },
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

pub mod agent_shader {
    vulkano_shaders::shader! {
        ty: "compute",
        src:
"
#version 450

const float PI = 3.1415926535897932384626433832795;

struct Agent {
    vec2 pos;
    float angle;
};

layout(local_size_x = 64, local_size_y = 1, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8) uniform readonly image2D trail_img;
layout(set = 0, binding = 1, rgba8) uniform writeonly image2D out_img;
layout(set = 0, binding = 2) buffer Agents {
    Agent data[];
} buf;

layout(push_constant) uniform PushConstantData {
    float agent_speed;
    float delta_time;
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

float normalize_from_hash(uint hash_val) {
    return float(hash_val) / 4294967295.0;
}

void main() {
    uint id = gl_GlobalInvocationID.x;
    if (id >= buf.data.length()) {
        return;
    }
    
    int width = imageSize(out_img).x;
    int height = imageSize(out_img).y;

    Agent agent = buf.data[id];
    uint random = hash(uint(agent.pos.y * width + agent.pos.x + hash(id)));
    
    // Move agent according to angle and speed.
    vec2 unit_direction = vec2(cos(agent.angle), sin(agent.angle));
    vec2 new_pos = agent.pos + unit_direction * pc.agent_speed * pc.delta_time;
    
    // Randomly bounce if agent hits the sides.
    if (new_pos.x < 0 || new_pos.x >= width || new_pos.y < 0 || new_pos.y >= height) {
        new_pos.x = min(width - 0.01, max(0, new_pos.x));
        new_pos.y = min(height - 0.01, max(0, new_pos.y));

        buf.data[id].angle = normalize_from_hash(random) * 2 * PI;
    }
    
    
    buf.data[id].pos = new_pos;

    // Draw trail.
    imageStore(out_img, ivec2(agent.pos), vec4(1.0));
}
"
    }
}

pub mod blur_fade_shader {
    vulkano_shaders::shader! {
            ty: "compute",
            src:
"
#version 450

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

layout(set = 0, binding = 0, rgba8) uniform readonly image2D in_img;
layout(set = 0, binding = 1, rgba8) uniform writeonly image2D out_img;

void main() {
    int width = imageSize(in_img).x;
    int height = imageSize(in_img).y;
    
    // ---- Blur ----
    vec4 sum = vec4(0.0, 0.0, 0.0, 0.0);
    for (int x = -1; x <= 1; x++) {
        for (int y = -1; y <= 1; y++) {
            uint sampleX = gl_GlobalInvocationID.x + x;
            uint sampleY = gl_GlobalInvocationID.y + y;
            
            if (sampleX >= 0 && sampleX < width && sampleY >= 0 && sampleY < height) {
                sum += imageLoad(in_img, ivec2(sampleX, sampleY));
            }
        }
    }
    
    vec4 blurred = sum / 9;
    
    // ---- Fade ----
    vec4 faded = blurred * 0.99;
    
    imageStore(out_img, ivec2(gl_GlobalInvocationID.xy), faded);
}
"
    }
}
