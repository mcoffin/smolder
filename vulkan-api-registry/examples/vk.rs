extern crate vulkan_api_registry;
extern crate xml;

use std::fs;
use vulkan_api_registry::Registry;

fn main() {
    let include_all = |_: &str| true;
    let events = fs::File::open("../Vulkan-Docs/src/spec/vk.xml")
        .map(xml::reader::EventReader::new).unwrap();
    let registry = Registry::new(events.into_iter(), &include_all, &include_all).unwrap();
    println!("{:?}", &registry);
}
