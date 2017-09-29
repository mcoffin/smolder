extern crate xml;

use std::{ fs, io };
use xml::reader::XmlEvent;
use xml::reader::Result as XmlResult;

enum TypeInfo {
    Basetype(String, String),
}

struct Contents<'a, It: Iterator<Item=XmlResult<XmlEvent>> + 'a> {
    events: &'a mut It,
    depth: u32,
}

impl<'a, It: Iterator<Item=XmlResult<XmlEvent>>> Contents<'a, It> {
    pub fn new(events: &'a mut It) -> Contents<'a, It> {
        Contents {
            events: events,
            depth: 1,
        }
    }
}

impl<'a, It: Iterator<Item=XmlResult<XmlEvent>>> Iterator for Contents<'a, It> {
    type Item = XmlResult<XmlEvent>;
    fn next(&mut self) -> Option<XmlResult<XmlEvent>> {
        if self.depth < 1 {
            None
        } else {
            self.events.next().map(|r| r.map(|e| match e {
                e @ XmlEvent::StartElement { .. } => {
                    self.depth = self.depth + 1;
                    e
                },
                e @ XmlEvent::EndElement { .. } => {
                    self.depth = self.depth - 1;
                    e
                },
                e => e,
            }))
        }
    }
}

fn main() {
    let file = fs::File::open("Vulkan-Docs/src/spec/vk.xml")
        .map(io::BufReader::new)
        .unwrap();

    let mut events = xml::reader::EventReader::new(file).into_iter();
    for e in events {
        match e {
            Ok(XmlEvent::StartElement { ref name, .. }) if name.borrow().local_name == "types" => {
                for e in Contents::new(&mut events) {
                    println!("{:?}", &e);
                }
            },
            _ => {},
        }
    }
}
