use crate::system::System;
use imgui::{im_str, Condition, Window};

mod simulation;
mod system;

fn main() {
    println!("Hello, world!");

    let system = System::init("Slime Simulation");

    // ---- Computing to an image buffer ----

    let sim = simulation::Simulation::init(system.device.clone(), system.queue.clone());

    // ---- Window imgui loop ----

    system.main_loop(sim, move |_, parameters, ui| {
        // TODO: make ui for adjusting agent parameters in real time,
        //       that way you can quickly see the effects of a change.
        Window::new(im_str!("Hello World!"))
            .size([300.0, 110.0], Condition::FirstUseEver)
            .build(ui, || {
                ui.text(im_str!("Hello World!"));
                ui.input_float(im_str!("Speed (px/s)"), &mut parameters.agent_speed)
                    .build();
                ui.input_float(
                    im_str!("Turn speed (rad/s)"),
                    &mut parameters.agent_turn_speed,
                )
                .build();
            });
    })
}
