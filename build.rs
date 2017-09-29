#![recursion_limit = "128"]

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

mod xast {
    use std::{ iter, slice };

    #[derive(Clone, Debug)]
    pub struct Node {
        pub name: String,
        pub attributes: Vec<(String, String)>,
        pub contents: Vec<Content>,
    }

    impl Node {
        pub fn children(&self) -> iter::FilterMap<slice::Iter<Content>, fn(&Content) -> Option<&Node>> {
            self.contents.iter().filter_map(Content::child)
        }
    }

    #[derive(Clone, Debug)]
    pub enum Content {
        Text(String),
        Child(Node)
    }

    impl Content {
        #[inline(always)]
        pub fn child(&self) -> Option<&Node> {
            match self {
                &Content::Child(ref n) => Some(n),
                _ => None,
            }
        }
    }
}

use std::borrow::Cow;

#[derive(Debug)]
enum ParseError {
    UnexpectedEOF,
    XmlError(xml::reader::Error),
    Other(Cow<'static, str>),
}

type ParseResult<T> = Result<T, ParseError>;

fn parse_node<It: Iterator<Item=XmlResult<XmlEvent>>>(events: &mut It) -> ParseResult<xast::Node> {
    let start = try!(events.next().map(|r| r.map_err(|e| ParseError::XmlError(e))).unwrap_or(Err(ParseError::UnexpectedEOF)));
    let mut node = try!(match start {
        XmlEvent::StartElement { ref name, ref attributes, .. } => Ok({
            xast::Node {
                name: name.local_name.clone(),
                attributes: attributes.iter().map(|attr| {
                    (attr.name.local_name.clone(), attr.value.clone())
                }).collect(),
                contents: Vec::new(),
            }
        }),
        e => Err(ParseError::Other(format!("Expected start of element but found: {:?}", e).into())),
    });
    let mut events = Contents::new(events);
    loop {
        let should_parse = {
            let mut events = events.by_ref().peekable();
            let next_event = {
                match events.peek() {
                    Some(e) => e,
                    None => break,
                }
            };
            match next_event {
                &Ok(XmlEvent::StartElement { .. }) => true,
                _ => false,
            }
        };
        if should_parse {
            let child = try!(parse_node(&mut events));
            node.contents.push(xast::Content::Child(child));
        } else {
            let next_event = events.next().unwrap();
            match next_event {
                Ok(XmlEvent::Characters(s)) => {
                    node.contents.push(xast::Content::Text(s));
                },
                _ => {}
            }
        }
    }
    Ok(node)
}

fn handle_event<It: Iterator<Item=XmlResult<XmlEvent>>>(events: &mut It) -> bool {
    let next_event = events.next();
    if let Some(next_event) = next_event {
        match next_event {
            Ok(XmlEvent::StartElement { ref name, .. }) if name.local_name == "types" => {
                let mut contents = Contents::new(events).peekable();
                loop {
                    let should_parse = {
                        let evt = contents.peek();
                        if let Some(evt) = evt {
                            match evt {
                                &Ok(XmlEvent::StartElement { ref name, .. }) if name.local_name == "type" => {
                                    true
                                },
                                _ => false,
                            }
                        } else {
                            break;
                        }
                    };
                    if should_parse {
                        println!("{:?}", parse_node(&mut contents).unwrap());
                    } else {
                        contents.next();
                    }
                }
            }
            _ => {}
        }
        true
    } else {
        false
    }
}

fn handle_events<It: Iterator<Item=XmlResult<XmlEvent>>, F: Fn(&mut It) -> bool>(mut events: It, handle_event: F) {
    while handle_event(&mut events) {}
}

fn main() {
    let file = fs::File::open("Vulkan-Docs/src/spec/vk.xml")
        .map(io::BufReader::new)
        .unwrap();

    let reader = xml::reader::EventReader::new(file);
    handle_events(reader.into_iter(), &handle_event);
}
