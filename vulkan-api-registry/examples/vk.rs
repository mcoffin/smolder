extern crate vulkan_api_registry;
extern crate xml;

use std::fs;
use vulkan_api_registry::Registry;

fn main() {
    let include_all = |_: &str| true;
    let events = fs::File::open("../Vulkan-Docs/src/spec/vk.xml")
        .map(xml::reader::EventReader::new).unwrap();
    let registry = Registry::new(events.into_iter(), &include_all, &include_all).unwrap();
    println!("types:");
    for (_, ty) in registry.types.iter() {
        println!("  - {:?}", ty);
    }
    println!("features:");
    for feature in registry.features.iter() {
        println!("  - {}", feature.name);
    }
    println!("extensions:");
    for extension in registry.extensions.iter() {
        println!("  - {}", extension.name);
    }
}
