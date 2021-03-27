use crate::system::System;

mod system;

fn main() {
    println!("Hello, world!");

    let system = System::init();
    system.main_loop()
}
