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

    system.main_loop(sim, move |_, ui| {
        Window::new(im_str!("Hello World!"))
            .size([300.0, 110.0], Condition::FirstUseEver)
            .build(ui, || {
                ui.text(im_str!("Hello World!"));
                ui.button(im_str!("Generate new"), [200.0, 30.0]);
            });
    })
}
