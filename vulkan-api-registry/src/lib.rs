extern crate xml;

mod xmlutil;

use xml::reader::XmlEvent;
use xml::reader::Result as XmlResult;

#[derive(Debug)]
pub enum Type {
    Enum {
    },
    Struct {
    },
    Union {
    },
    Basetype {
    },
    Bitmask {
    },
    Define {
    },
    Funcpointer {
    },
    Group {
    },
    Handle {
    },
    Include {
    },
}

pub enum RegistryEntry {
    Type(Type),
}

pub fn parse_registry<It: Iterator<Item=XmlResult<XmlEvent>>>(events: It) -> XmlResult<Vec<RegistryEntry>> {
    let event_matches = |e: XmlEvent| match e {
        XmlEvent::StartElement { ref name, .. } if name.borrow().local_name == "type" => true,
        _ => false,
    };
    let mut events = events.skip_while(|r| !r.clone().ok().map(&event_matches).unwrap_or(false));
    events.next().ok().flat_map(|v| v).map(|_event| {
        let contents = xmlutil::Contents::new(&mut events);
        let entry = try!(parse_registry_entry(contents));
        let mut other_entries = parse_registry(
    })
    Ok(ret)
}

pub fn parse_registry_entry<It: Iterator<Item=XmlResult<XmlEvent>>>(mut events: It) -> XmlResult<RegistryEntry> {
    for e in events {
        println!("{:?}", &e);
        try!(e);
    }
    Ok(RegistryEntry::Type(Type::Basetype {}))
}
