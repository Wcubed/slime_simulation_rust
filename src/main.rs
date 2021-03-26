use vulkano::instance::{Instance, InstanceExtensions, PhysicalDevice};

fn main() {
    println!("Hello, world!");

    let instance =
        Instance::new(None, &InstanceExtensions::none(), None).expect("Failed to create instance.");

    let physical = PhysicalDevice::enumerate(&instance)
        .next()
        .expect("No device available");

    // Debug print all the queue families.
    for family in physical.queue_families() {
        println!(
            "Found a queue family with {:?} queues. Supports graphics: {:?}, compute: {:?}",
            family.queues_count(),
            family.supports_graphics(),
            family.supports_compute()
        );
    }
}
