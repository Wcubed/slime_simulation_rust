use crate::system::System;
use imgui::{im_str, Condition, Slider, Window};
use std::f32::consts::PI;

mod simulation;
mod system;

fn main() {
    println!("Hello, world!");

    let system = System::init("Slime Simulation");

    // ---- Computing to an image buffer ----

    let sim = simulation::Simulation::init(system.device.clone(), system.queue.clone());

    // ---- Window imgui loop ----

    system.main_loop(sim, move |_, sim_parameters, fade_parameters, ui| {
        Window::new(im_str!("Hello World!"))
            .size([300.0, 200.0], Condition::FirstUseEver)
            .build(ui, || {
                ui.push_item_width(100.0);

                ui.text(im_str!("Hello World!"));
                ui.input_float(im_str!("Speed (px/s)"), &mut sim_parameters.agent_speed)
                    .build();
                ui.input_float(
                    im_str!("Turn speed (rad/s)"),
                    &mut sim_parameters.agent_turn_speed,
                )
                .build();
                ui.input_int(im_str!("Sensor radius"), &mut sim_parameters.sensor_radius)
                    .build();
                Slider::new(im_str!("Sensor angles"))
                    .range(0.0..=PI)
                    .build(&ui, &mut sim_parameters.sensor_angle_spacing);
                ui.input_float(im_str!("Fade speed"), &mut fade_parameters.evaporate_speed)
                    .build();
            });
    })
}
