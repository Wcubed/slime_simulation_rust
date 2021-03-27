use crate::system::System;
use imgui::{im_str, Window};

mod system;

fn main() {
    println!("Hello, world!");

    let system = System::init("Slime Simulation");
    system.main_loop(move |_, ui| {
        Window::new(im_str!("Hello World!"));
    })
}
