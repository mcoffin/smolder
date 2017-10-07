use std::borrow::Cow;
use std::collections::LinkedList;
use xml::attribute::OwnedAttribute;
use xml::reader::XmlEvent;
use xml::reader::Result as XmlResult;
use ::{ ParseError, ParseResult, XmlParse };
use ::xml_iter::XmlContents;

#[derive(Debug, Clone)]
pub enum Content {
    Comment(String),
    Text(String),
    Child(Node),
}

#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub attributes: Vec<OwnedAttribute>,
    pub contents: LinkedList<Content>,
}

impl Node {
    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        self.attributes.iter()
            .find(|attr| attr.name.local_name == name)
            .map(|attr| attr.value.as_str())
    }

    pub fn get_attribute_or_child(&self, name: &str) -> Option<Cow<str>> {
        self.get_attribute(name)
            .map(Into::into)
            .or_else(|| self.get_child(name).map(|child| child.concat_text().into()))
    }

    pub fn get_child(&self, name: &str) -> Option<&Node> {
        self.contents.iter().filter_map(|c| match c {
            &Content::Child(ref n) => Some(n),
            _ => None
        }).find(|c| c.name == name)
    }

    pub fn concat_text(&self) -> String {
        let mut ret: String = "".into();
        let text_contents = self.contents.iter().filter_map(|c| match c {
            &Content::Text(ref s) => Some(s),
            _ => None,
        });
        for s in text_contents {
            ret.push_str(s.as_str());
        }
        ret
    }
}

impl XmlParse for Node {
    fn parse<It: Iterator<Item=XmlResult<XmlEvent>>>(events: It, name: String, attributes: Vec<OwnedAttribute>) -> ParseResult<Node> {
        let events = XmlContents::new_inside(events);
        let mut node_stack: LinkedList<Node> = LinkedList::new();
        node_stack.push_front(Node {
            name: name,
            attributes: attributes,
            contents: LinkedList::new(),
        });
        for e in events {
            match try!(e.map_err(|e| ParseError::Xml(e))) {
                XmlEvent::StartElement { name, attributes, .. } => {
                    node_stack.push_front(Node {
                        name: name.local_name,
                        attributes: attributes,
                        contents: LinkedList::new(),
                    });
                },
                XmlEvent::EndElement { .. } => {
                    let child = node_stack.pop_front().unwrap();
                    let node = node_stack.front_mut().unwrap();
                    node.contents.push_back(Content::Child(child));
                },
                XmlEvent::Characters(s) => {
                    let node = node_stack.front_mut().unwrap();
                    node.contents.push_back(Content::Text(s));
                },
                XmlEvent::Comment(s) => {
                    let node = node_stack.front_mut().unwrap();
                    node.contents.push_back(Content::Comment(s));
                },
                _ => {},
            }
        }
        if node_stack.len() == 1 {
            Ok(node_stack.pop_front().unwrap())
        } else {
            Err(ParseError::UnexpectedEOF)
        }
    }
}
