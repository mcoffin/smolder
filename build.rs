extern crate xml;

use std::{ env, fs, io, path };
use xml::reader::XmlEvent;
use xml::reader::Result as XmlResult;

mod xast;

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
    Handle {
        name: String,
        dispatchable: bool,
    },
    Struct {
        name: String,
        members: Vec<(String, String)>
    },
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
    let name = name.expect("Basetype did not have a name");
    let ty = ty.expect("Basetype did not have a type");
    Ok(TypeInfo::Basetype(name, ty))
}

fn get_attribute<'a, It: Iterator<Item=&'a xml::attribute::OwnedAttribute>>(attribute: &str, mut attributes: It) -> Option<&'a str> {
    attributes
        .find(|attr| attr.name.local_name == attribute)
        .map(|attr| attr.value.as_str())
}

fn read_node<It: Iterator<Item=XmlResult<XmlEvent>>>(mut events: It, attributes: Vec<xml::attribute::OwnedAttribute>) -> XmlResult<xast::Node> {
    let mut events = Contents::new(&mut events);
    let mut node = xast::Node {
        attributes: attributes,
        contents: Vec::new(),
    };
}

fn read_next_member<It: Iterator<Item=XmlResult<XmlEvent>>>(mut events: It) -> Option<XmlResult<(String, String)>> {
    let mut events = Contents::new(&mut events);
    let next_event = {
        let mut events = events.by_ref().skip_while(|evt| match evt {
            &Err(_) => false,
            &Ok(XmlEvent::StartElement { ref name, .. }) if name.borrow().local_name == "member" => false,
            &Ok(_) => true,
        });
        events.next()
    };
    next_event.map(|r| r.and_then(|evt| match evt {
        XmlEvent::StartElement { attributes } => {
        },
        _ => unreachable!(),
    }))
    //next_event.map(|r| r.and_then(|_| read_basetype(&mut events).map(|t| match t {
    //    TypeInfo::Basetype(name, ty) => (name, ty),
    //    _ => unreachable!(),
    //})))
}

fn read_struct<It: Iterator<Item=XmlResult<XmlEvent>>>(mut events: It, name: String) -> XmlResult<TypeInfo> {
    let mut events = Contents::new(&mut events);
    let members = FromNextFn::new(|| read_next_member(&mut events));
    let mut members_v: Vec<(String, String)> = Vec::new();
    for member in members {
        members_v.push(try!(member));
    }
    Ok(TypeInfo::Struct {
        name: name,
        members: members_v,
    })
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
                        "handle" => {
                            read_basetype(&mut events).map(|info| match info {
                                TypeInfo::Basetype(name, hty) => {
                                    let dispatchable = hty == "VK_DEFINE_HANDLE";
                                    Some(TypeInfo::Handle {
                                        name: name,
                                        dispatchable: dispatchable,
                                    })
                                },
                                _ => unreachable!(),
                            })
                        },
                        "struct" => {
                            if let Some(name) = get_attribute("name", attributes.iter()) {
                                read_struct(&mut events, name.into()).map(|info| Some(info))
                            } else {
                                Ok(None)
                            }
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
    let mut events = Contents::new(&mut events);
    let types = FromNextFn::new(|| read_type(&mut events));
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

struct FromNextFn<A, F: FnMut() -> Option<A>> {
    f: F,
}

impl<A, F: FnMut() -> Option<A>> FromNextFn<A, F> {
    pub fn new(f: F) -> Self {
        FromNextFn {
            f: f,
        }
    }
}

impl<A, F: FnMut() -> Option<A>> Iterator for FromNextFn<A, F> {
    type Item = A;
    fn next(&mut self) -> Option<A> {
        (self.f)()
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
    let mut events = reader.into_iter();
    for e in FromNextFn::new(|| read_top_level(&mut events)) {
        match e.unwrap() {
            TopLevelElement::Types(types) => {
                write!(&mut out_file, "// types\n").unwrap();
                for t in types.into_iter() {
                    match t {
                        TypeInfo::Basetype(name, ty) => {
                            write!(&mut out_file, "pub type {} = {};\n", &name, &ty).unwrap();
                        },
                        TypeInfo::Handle { name, dispatchable } => {
                            let macro_name = if dispatchable {
                                "smolder_ffi_handle"
                            } else {
                                "smolder_ffi_handle_nondispatchable"
                            };
                            write!(&mut out_file, "{}!({});\n", macro_name, &name).unwrap();
                        },
                        TypeInfo::Struct { name, members } => {
                            write!(&mut out_file, "#[repr(C)]\n#[derive(Debug)]\n").unwrap();
                            write!(&mut out_file, "pub struct {} {{\n", &name).unwrap();
                            for (name, ty) in members {
                                write!(&mut out_file, "    {}: {},\n", &name, &ty).unwrap();
                            }
                            out_file.write_all(b"}\n").unwrap();
                        },
                    }
                }
            }
        }
    }
}
