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

    system.main_loop(sim.image.clone(), move |_, ui| {
        Window::new(im_str!("Hello World!"))
            .size([300.0, 110.0], Condition::FirstUseEver)
            .build(ui, || {
                ui.text(im_str!("Hello World!"));
                let clicked = ui.button(&im_str!("Run computation!"), [300.0, 30.0]);

                if clicked {
                    sim.run_once();
                }
            });
    })
}
