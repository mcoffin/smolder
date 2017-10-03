extern crate xml;

use std::{ env, fs, io, path };
use xml::reader::XmlEvent;
use xml::reader::Result as XmlResult;

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

    pub fn depth(&self) -> u32 {
        self.depth
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

#[derive(Debug)]
enum TypeInfo {
    Basetype(String, String),
}

#[derive(Debug)]
enum TopLevelElement {
    Types(Vec<TypeInfo>),
}

const TOP_LEVEL_NAMES: &'static [&'static str] = &[
    "types"
];

#[derive(Clone, Copy)]
enum ReadBasetypeState {
    Scanning,
    ReadType(u32),
    ReadName(u32),
}

impl Default for ReadBasetypeState {
    fn default() -> ReadBasetypeState {
        ReadBasetypeState::Scanning
    }
}

fn read_basetype<It: Iterator<Item=XmlResult<XmlEvent>>>(mut events: It) -> XmlResult<TypeInfo> {
    use ReadBasetypeState::*;

    let events = Contents::new(&mut events);
    let mut state: ReadBasetypeState = Default::default();
    let mut name: Option<String> = Default::default();
    let mut ty: Option<String> = Default::default();
    for e in events {
        match (state, try!(e)) {
            (Scanning, XmlEvent::StartElement { ref name, .. }) => {
                let name = name.borrow().local_name;
                match name {
                    "type" => {
                        state = ReadType(1);
                    },
                    "name" => {
                        state = ReadName(1);
                    },
                    _ => {},
                }
            },
            (Scanning, _) => {},
            (ReadName(1), XmlEvent::Characters(s)) => {
                match name {
                    Some(ref mut name) => {
                        name.push_str(s.as_str());
                    },
                    None => {
                        name = Some(s);
                    },
                }
            },
            (ReadName(depth), XmlEvent::StartElement { .. }) => {
                state = ReadName(depth + 1)
            },
            (ReadName(depth), XmlEvent::EndElement { .. }) => {
                if depth == 1 {
                    state = Scanning;
                } else {
                    state = ReadName(depth - 1);
                }
            },
            (ReadName(_), _) => {},
            (ReadType(1), XmlEvent::Characters(s)) => {
                match ty {
                    Some(ref mut ty) => {
                        ty.push_str(s.as_str());
                    },
                    None => {
                        ty = Some(s);
                    },
                }
            },
            (ReadType(depth), XmlEvent::StartElement { .. }) => {
                state = ReadType(depth + 1)
            },
            (ReadType(depth), XmlEvent::EndElement { .. }) => {
                if depth == 1 {
                    state = Scanning;
                } else {
                    state = ReadType(depth - 1);
                }
            },
            (ReadType(_), _) => {},
        }
    }
    Ok(TypeInfo::Basetype(name.unwrap(), ty.unwrap()))
}

fn get_attribute<'a, It: Iterator<Item=&'a xml::attribute::OwnedAttribute>>(attribute: &str, mut attributes: It) -> Option<&'a str> {
    attributes
        .find(|attr| attr.name.local_name == attribute)
        .map(|attr| attr.value.as_str())
}

fn read_type<It: Iterator<Item=XmlResult<XmlEvent>>>(mut events: It) -> Option<XmlResult<TypeInfo>> {
    loop {
        let next_event = {
            let mut events = events.by_ref().skip_while(|e| match e {
                &Err(_) => false,
                &Ok(XmlEvent::StartElement { ref name, .. }) if name.borrow().local_name == "type" => false,
                &Ok(_) => true,
            });
            events.next()
        };
        let ret = next_event.map(|r| r.and_then(|evt| match evt {
            XmlEvent::StartElement { ref attributes, .. } => {
                if let Some(category) = get_attribute("category", attributes.iter()) {
                    match category {
                        "basetype" => {
                            read_basetype(&mut events).map(|info| Some(info))
                        },
                        _ => {
                            Ok(None)
                        }
                    }
                } else {
                    Ok(None)
                }
            },
            _ => unreachable!(),
        }));
        match ret {
            Some(Ok(Some(type_info))) => {
                return Some(Ok(type_info));
            },
            Some(Ok(None)) => {
                // Keep going, until we encounter an error, the end of the stream, or we get a type
            },
            Some(Err(e)) => {
                return Some(Err(e));
            },
            None => {
                return None;
            },
        }
    }
}

fn read_types<It: Iterator<Item=XmlResult<XmlEvent>>>(mut events: It) -> XmlResult<TopLevelElement> {
    // We only want to read the contents of this tag.
    let mut v: Vec<TypeInfo> = Vec::new();
    let types = FromNextFn::new(Contents::new(&mut events), |it| read_type(it));
    for t in types {
        v.push(try!(t));
    }
    Ok(TopLevelElement::Types(v))
}

fn read_top_level<It: Iterator<Item=XmlResult<XmlEvent>>>(events: &mut It) -> Option<XmlResult<TopLevelElement>> {
    // Ideally we'd just implement this with tail call optimized recursion but for some reason
    // that didn't work so we do a `skip_while` and then use unreachable!() blocks below.
    let next_event = {
        let mut events = events.skip_while(|e| match e {
            &Err(_) => false,
            &Ok(XmlEvent::StartElement { ref name, .. }) => !TOP_LEVEL_NAMES.contains(&name.borrow().local_name),
            &Ok(_) => true,
        });
        events.next()
    };
    next_event.map(|r| r.and_then(|evt| match evt {
        XmlEvent::StartElement { ref name, .. } => match name.borrow().local_name {
            "types" => read_types(events),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }))
}

struct FromNextFn<A, F: FnMut(&mut It) -> Option<A>, It> {
    it: It,
    f: F,
}

impl<A, F: FnMut(&mut It) -> Option<A>, It> FromNextFn<A, F, It> {
    pub fn new(it: It, f: F) -> Self {
        FromNextFn {
            it: it,
            f: f,
        }
    }
}

impl<A, F: FnMut(&mut It) -> Option<A>, It> Iterator for FromNextFn<A, F, It> {
    type Item = A;
    fn next(&mut self) -> Option<A> {
        (self.f)(&mut self.it)
    }
}

const VK_SPEC_PATH: &'static str = "Vulkan-Docs/src/spec/vk.xml";

fn main() {
    use io::Write;

    let mut out_file = {
        let dest_path = env::var("OUT_DIR")
            .map(|out_dir| path::Path::new(&out_dir).join("vk.rs"))
            .unwrap();
        fs::File::create(dest_path).unwrap()
    };

    let reader = fs::File::open(VK_SPEC_PATH)
        .map(io::BufReader::new)
        .map(xml::reader::EventReader::new)
        .unwrap();
    for e in FromNextFn::new(reader.into_iter(), read_top_level) {
        match e.unwrap() {
            TopLevelElement::Types(types) => {
                write!(&mut out_file, "// types\n").unwrap();
                for t in types.into_iter() {
                    match t {
                        TypeInfo::Basetype(name, ty) => {
                            write!(&mut out_file, "pub type {} = {};\n", &name, &ty).unwrap();
                        }
                    }
                }
            }
        }
    }
}
