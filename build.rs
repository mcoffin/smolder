extern crate xml;

use std::{ fs, io };
use std::collections::VecDeque;
use xml::attribute::OwnedAttribute;
use xml::namespace::Namespace;
use xml::reader::XmlEvent;

enum TypeInfo {
    Basetype(String, String),
}

fn parse_type(&mut xml::reader::Events, attributes: &Vec<OwnedAttribute>, &Namespace) -> xml::reader::Result<TypeInfo> {
}

fn parse_all<T, F: Fn(&mut xml::reader::Events, &Vec<OwnedAttribute>, &Namespace) -> xml::reader::Result<T>>(parser: &mut xml::reader::Events, tag_name: &str, parse_fn: F) -> xml::reader::Result<Vec<T>> {
    let mut depth: u32 = 1;
    let items: Vec<T> = Vec::new();
    for &e in parser {
        match try!(e) {
            XmlEvent::StartElement { ref name, ref attributes, ref namespace } => {
                if depth == 1 && name.borrow().local_name == tag_name {
                    let item = try!(parse_fn(parser), name, attributes, namespace);
                    items.push(item);
                } else {
                    depth = depth + 1;
                }
            },
            XmlEvent::EndElement { .. } => {
                depth -= 1;
                if depth <= 0 {
                    break;
                }
            },
            _ => {},
        }
    }
    Ok(items)
}

fn main() {
    let file = fs::File::open("vk.xml")
        .map(io::BufReader::new)
        .unwrap();

    let mut state: VecDeque<ParseState> = VecDeque::new();
    state.push_back(Default::default());

    let mut parser = xml::reader::EventReader::new(file).into_iter();
    while let Some(Ok(e)) = parser.next() {
        e match {
            XmlEvent::StartElement { ref name, .. } if name.borrow().local_name == "types" => {
                parse_all(&mut parser, "type", parse_type
            },
        }
    }
}
