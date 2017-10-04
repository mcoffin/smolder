use std::{ iter, vec };
use xml::attribute::OwnedAttribute;

#[derive(Debug, Clone)]
pub enum Content {
    Text(String),
    Child(Node),
}

#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub attributes: Vec<OwnedAttribute>,
    pub contents: Vec<Content>,
}

impl Node {
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
