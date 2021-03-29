use rand::Rng;
use std::f32::consts::PI;
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBuffer, AutoCommandBufferBuilder};
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
    agent_amount: u32,
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
            width: 2000,
            height: 1024,
        };
        let agent_amount = 200000;

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

        // Distribute the agents randomly across the image.
        let agent_iter = (0..agent_amount).map(|_i| agent_shader::ty::Agent {
            // No clue what the dummy is for.
            _dummy0: [0u8; 4],
            pos: [
                rng.gen_range(0..image_size.width()) as f32,
                rng.gen_range(0..image_size.height()) as f32,
            ],
            angle: rng.gen::<f32>() * 2.0 * PI,
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
            agent_amount,
            agent_sim_pipeline,
            agent_sim_set,
            agent_sim_image,
            blur_pipeline,
            blur_set,
        }
    }

    /// The command buffers should be executed in the order given.
    pub fn create_command_buffers(
        &self,
        sim_parameters: &agent_shader::ty::PushConstantData,
        fade_parameters: &blur_fade_shader::ty::PushConstantData,
    ) -> (AutoCommandBuffer, AutoCommandBuffer, AutoCommandBuffer) {
        let mut copy_builder =
            AutoCommandBufferBuilder::new(self.device.clone(), self.queue.family())
                .expect("Failed to create command buffer");
        // Transfer old trails.
        copy_builder
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
            .unwrap();
        let copy_buffer = copy_builder.build().unwrap();

        let mut sim_builder =
            AutoCommandBufferBuilder::new(self.device.clone(), self.queue.family())
                .expect("Failed to create command buffer");
        sim_builder
            .dispatch(
                [self.agent_amount / 64, 1, 1],
                self.agent_sim_pipeline.clone(),
                self.agent_sim_set.clone(),
                sim_parameters.clone(),
            )
            .unwrap();
        let sim_buffer = sim_builder.build().unwrap();

        let mut blur_builder =
            AutoCommandBufferBuilder::new(self.device.clone(), self.queue.family())
                .expect("Failed to create command buffer");
        blur_builder
            .dispatch(
                [
                    self.result_image.dimensions().width() / 8,
                    self.result_image.dimensions().height() / 8,
                    1,
                ],
                self.blur_pipeline.clone(),
                self.blur_set.clone(),
                fade_parameters.clone(),
            )
            .unwrap();
        let blur_buffer = blur_builder.build().unwrap();

        (copy_buffer, sim_buffer, blur_buffer)
    }
}

pub mod agent_shader {
    vulkano_shaders::shader! {
        ty: "compute",
        src:
"
#version 450

const float PI = 3.1415926535897932384626433832795;
const float HALF_PI = PI / 0.5;

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
    // In pixels / second.
    float agent_speed;
    // In radians / second.
    float agent_turn_speed;
    int sensor_radius;
    // In radians from straight ahead.
    float sensor_angle_spacing;
    // How many time is passed per frame.
    float delta_time;
} pc;

int width = imageSize(out_img).x;
int height = imageSize(out_img).y;

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


float sense(Agent agent, float sensor_angle_offset) {
    float sensor_centre_distance = 9.0;
    
    float sensor_angle = agent.angle + sensor_angle_offset;
    vec2 sensor_dir_norm = vec2(cos(sensor_angle), sin(sensor_angle));    
    ivec2 sensor_centre = ivec2(agent.pos + (sensor_dir_norm * sensor_centre_distance));
    
    float sum = 0;
    for (int x = -pc.sensor_radius; x <= pc.sensor_radius; x++) {
        for (int y = -pc.sensor_radius; y <= pc.sensor_radius; y++) {
            ivec2 sample_pos = ivec2(sensor_centre.x + x, sensor_centre.y + y);

            if (sample_pos.x >= 0 && sample_pos.x < width && sample_pos.y >= 0 && sample_pos.y < height) {
                vec4 value = imageLoad(trail_img, sample_pos);
                sum += value.x + value.y + value.z;
            }
        }
    }
    
    // TODO: Remove debug.
    /*if (sum > 0) {
        imageStore(out_img, sensor_centre, vec4(0.0, sum, 0.0, 1.0));
    }*/
    
    return sum;
}


void main() {
    uint id = gl_GlobalInvocationID.x;
    if (id < 0 || id >= buf.data.length()) {
        return;
    }
    
    int width = imageSize(out_img).x;
    int height = imageSize(out_img).y;

    Agent agent = buf.data[id];
    uint random = hash(uint(agent.pos.y * width + agent.pos.x + hash(id)));
    
    // Decide which way to turn.
    float sense_forward = sense(agent, 0);
    float sense_left = sense(agent, pc.sensor_angle_spacing);
    float sense_right = sense(agent, -pc.sensor_angle_spacing);
    
    float random_steer_strength = normalize_from_hash(random);
    
    if (sense_forward > sense_left && sense_forward > sense_right) {
        // Continue straight.
    } else if (sense_forward < sense_left && sense_forward < sense_right) {
        // Don't know whether to go left or right? Go random.
        buf.data[id].angle += (random_steer_strength - 0.5) * 2 * pc.agent_turn_speed * pc.delta_time;
    } else if (sense_left > sense_right) {
        // Go left.
        buf.data[id].angle += random_steer_strength * pc.agent_turn_speed * pc.delta_time;
    } else if (sense_left < sense_right) {
        // Go right.
        buf.data[id].angle -= random_steer_strength * pc.agent_turn_speed * pc.delta_time;
    }
    
    // Move agent according to angle and speed.
    vec2 unit_direction = vec2(cos(agent.angle), sin(agent.angle));
    vec2 new_pos = agent.pos + unit_direction * pc.agent_speed * pc.delta_time;
    
    // How far to move from the edge when bouncing against it.
    float edge_holdout = 0.01;
    
    bool top = new_pos.y < 0;
    bool bottom = new_pos.y >= height;
    bool left = new_pos.x < 0;
    bool right = new_pos.x >= width;
    
    // Randomly bounce if agent hits the corners or the sides.
    // Never bounce into the side, always away from it.
    if (bottom && left) {
        new_pos.x = edge_holdout;
        new_pos.y = edge_holdout;
        buf.data[id].angle = normalize_from_hash(random) * -HALF_PI;
    } else if (bottom && right) {
        new_pos.x = width - edge_holdout;
        new_pos.y = edge_holdout;
        buf.data[id].angle = normalize_from_hash(random) * HALF_PI - PI;
    } else if (top && left) {
        new_pos.x = edge_holdout;
        new_pos.y = height - edge_holdout;
        buf.data[id].angle = normalize_from_hash(random) * HALF_PI;
    } else if (top && right) {
        new_pos.x = width - edge_holdout;
        new_pos.y = height - edge_holdout;
        buf.data[id].angle = normalize_from_hash(random) * HALF_PI + HALF_PI;
    } else if (left) {
        new_pos.x = edge_holdout;
        buf.data[id].angle = normalize_from_hash(random) * PI - HALF_PI;
    } else if (right) {
        new_pos.x = width - edge_holdout;
        buf.data[id].angle = normalize_from_hash(random) * PI + HALF_PI;
    } else if (top) {
        new_pos.y = edge_holdout;
        buf.data[id].angle = normalize_from_hash(random) * PI;
    } else if (bottom) {
        new_pos.y = height - edge_holdout;
        buf.data[id].angle = normalize_from_hash(random) * -PI;
    }
    
    
    buf.data[id].pos = new_pos;

    // Draw trail.
    imageStore(out_img, ivec2(agent.pos), vec4(1.0, 0.0, 0.5, 1.0));
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

layout(push_constant) uniform PushConstantData {
    // How many time is passed per frame.
    float delta_time;
    // How much color is 'evaporated' per second.
    float evaporate_speed;
} pc;

void main() {
    int width = imageSize(in_img).x;
    int height = imageSize(in_img).y;
    
    int blur_radius = 1;
    
    // ---- Blur ----
    vec4 sum = vec4(0.0, 0.0, 0.0, 0.0);
    for (int x = -blur_radius; x <= blur_radius; x++) {
        for (int y = -blur_radius; y <= blur_radius; y++) {
            ivec2 sample_pos = ivec2(gl_GlobalInvocationID.x + x, gl_GlobalInvocationID.y + y);
            
            if (sample_pos.x >= 0 && sample_pos.x < width && sample_pos.y >= 0 && sample_pos.y < height) {
                sum += imageLoad(in_img, sample_pos);
            }
        }
    }
    
    vec4 blurred = sum / ((blur_radius * 2 + 1) * (blur_radius * 2 + 1));
    
    // ---- Evaporate ----
    vec4 result = vec4(max(0.0, blurred.x - pc.evaporate_speed * pc.delta_time),
                    max(0.0, blurred.y - pc.evaporate_speed * pc.delta_time),
                    max(0.0, blurred.z - pc.evaporate_speed * pc.delta_time),
                    max(0.0, blurred.w - pc.evaporate_speed * pc.delta_time));
    
    imageStore(out_img, ivec2(gl_GlobalInvocationID.xy), result);
}
"
    }
}
