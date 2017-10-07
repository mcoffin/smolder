extern crate regex;
extern crate xml;

mod iter_util;
mod xast;
mod xml_iter;

use iter_util::FromNextFn;
use regex::Regex;
use std::borrow::Cow;
use std::collections::{ BTreeSet, HashMap, LinkedList };
use xml::attribute::OwnedAttribute;
use xml::reader::XmlEvent;
use xml::reader::Result as XmlResult;
use xml_iter::XmlContents;

#[derive(Debug, Clone)]
pub enum ParseError {
    UnexpectedEOF,
    Custom(Cow<'static, str>),
    Xml(xml::reader::Error),
}

pub type ParseResult<T> = Result<T, ParseError>;

pub trait XmlParse: Sized {
    fn parse<It: Iterator<Item=XmlResult<XmlEvent>>>(events: It, name: String, attributes: Vec<OwnedAttribute>) -> ParseResult<Self>;
}

#[derive(Debug, Clone)]
pub struct TyperefInfo {
    pub ty: String,
    pub constness: Vec<bool>,
}

#[derive(Debug, Clone)]
pub enum EnumValue {
    Bitpos(u8),
    Value(isize),
    StrValue(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleType {
    Normal,
    NonDispatchable,
}

#[derive(Debug, Clone)]
pub struct StructMember {
    pub name: String,
    pub ty: TyperefInfo,
    pub values: Option<LinkedList<String>>,
    pub externsync: bool,
    pub len: Option<String>,
    pub altlen: Option<String>,
    pub optional: bool,
    pub noautovalidity: bool,
}

#[derive(Debug, Clone)]
pub enum TypeInfo {
    Basetype {
        name: String,
        ty: String,
    },
    Bitmask {
        name: String,
        ty: String,
    },
    Define(String), // TODO: unused rn for rust
    Enum {
        name: String,
        values: LinkedList<(String, EnumValue)>,
    },
    Constant {
        name: String,
        value: EnumValue,
    },
    Funcpointer {
        name: String,
        arguments: LinkedList<(String, TyperefInfo)>,
        return_type: TyperefInfo,
    },
    Group, // TODO: unused rn
    Handle {
        name: String,
        parent: Option<String>,
        ty: HandleType,
    },
    Struct {
        name: String,
        members: LinkedList<StructMember>,
        extends: LinkedList<String>,
    },
    Include(String), // TODO: unused rn
    Uncategorized {
        name: String,
        node: xast::Node,
    },
}

impl TypeInfo {
    fn name(&self) -> &str {
        use TypeInfo::*;
        let s = match self {
            &Basetype { ref name, .. } => name,
            &Bitmask { ref name, .. } => name,
            &Define(ref name) => name,
            &Enum { ref name, .. } => name,
            &Constant { ref name, .. } => name,
            &Funcpointer { ref name, .. } => name,
            &Group => unimplemented!(),
            &Handle { ref name, .. } => name,
            &Struct { ref name, .. } => name,
            &Include(ref name) => name,
            &Uncategorized { ref name, .. } => name,
        };
        s.as_str()
    }
    pub fn parse_type<It: Iterator<Item=XmlResult<XmlEvent>>>(events: It, name: String, attributes: Vec<OwnedAttribute>) -> ParseResult<TypeInfo> {
        let node: xast::Node = try!(XmlParse::parse(events, name, attributes));
        let name = try! {
            node.get_attribute_or_child("name")
                .map(|s| Ok(s))
                .unwrap_or_else(|| Err(ParseError::Custom("Type did not have a name".into())))
        };
        let info = match node.get_attribute("category") {
            Some("basetype") => TypeInfo::Basetype {
                name: name.into(),
                ty: try! {
                    node.get_attribute("type")
                        .map(|ty| Ok(ty.into()))
                        .unwrap_or(Err(ParseError::Custom("basetype did not have a type tag".into())))
                },
            },
            Some("bitmask") => TypeInfo::Bitmask {
                name: name.into(),
                ty: try! {
                    node.get_attribute("type")
                        .map(|ty| Ok(ty.into()))
                        .unwrap_or(Err(ParseError::Custom("bitmask did not have a type tag".into())))
                },
            },
            Some("define") => TypeInfo::Define(name.into()),
            Some("enum") => TypeInfo::Enum {
                name: name.into(),
                values: LinkedList::new()
            },
            Some("funcpointer") => unimplemented!(),
            Some("group") => TypeInfo::Group,
            Some("handle") => {
                let ty = try! {
                    node.get_child("type")
                        .map(|c| Ok(c.concat_text()))
                        .unwrap_or_else(|| Err(ParseError::Custom("No type tag for handle".into())))
                        .map(|ty| {
                            if ty == "VK_DEFINE_HANDLE" {
                                HandleType::Normal
                            } else {
                                HandleType::NonDispatchable
                            }
                        })
                };
                TypeInfo::Handle {
                    name: name.into(),
                    parent: node.get_attribute("parent").map(Into::into),
                    ty: ty,
                }
            },
            Some("include") => TypeInfo::Include(name.into()),
            Some(category) => {
                return Err(ParseError::Custom(format!("Unknown type category: {}", category).into()));
            },
            None => TypeInfo::Uncategorized {
                name: name.into(),
                node: node.clone(),
            },
        };
        Ok(info)
    }
    pub fn parse_next_type<It: Iterator<Item=XmlResult<XmlEvent>>>(mut events: It) -> Option<ParseResult<TypeInfo>> {
        // TODO: this should be implementable with tail call recursion
        let next_event = {
            let mut events = events.by_ref().skip_while(|evt| match evt {
                &Err(_) => false,
                &Ok(XmlEvent::StartElement { ref name, .. }) => match name.local_name.as_str() {
                    "type" => false,
                    _ => true,
                },
                &Ok(_) => true,
            });
            events.next()
        };
        next_event.map(|r| r.map_err(|e| ParseError::Xml(e)).and_then(|evt| match evt {
            XmlEvent::StartElement { name, attributes, .. } => TypeInfo::parse_type(&mut events, name.local_name, attributes),
            _ => unreachable!(),
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRenderPassInfo {
    Inside,
    Outside,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandBufferLevel {
    Primary,
    Secondary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandPipelineType {
    Compute,
    Transfer,
    Graphics,
}

#[derive(Debug, Clone)]
pub struct ParameterInfo {
    pub name: String,
    pub ty: TyperefInfo,
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub name: String,
    pub queues: LinkedList<String>,
    pub successcodes: LinkedList<String>,
    pub errorcodes: LinkedList<String>,
    pub renderpass: Option<CommandRenderPassInfo>,
    pub cmdbufferlevel: Option<BTreeSet<CommandBufferLevel>>,
    pub pipeline: Option<CommandPipelineType>,
    pub return_type: TyperefInfo,
    pub params: LinkedList<ParameterInfo>,
}

#[derive(Debug, Clone)]
pub enum EnumExtensionStrategy {
    Value(EnumValue),
    Offset {
        offset: isize,
        negated: bool,
    },
}

#[derive(Debug, Clone)]
pub enum Requirement {
    Type(String),
    Command(String),
    Enum(String),
    Constant {
        name: String,
        value: EnumValue,
        ty: Option<String>,
    },
    EnumExtension {
        name: String,
        extends: String,
        value: EnumExtensionStrategy,
    },
}

impl Requirement {
    pub fn parse_next_requirement<It: Iterator<Item=XmlResult<XmlEvent>>>(events: It) -> Option<ParseResult<Requirement>> {
        events.filter_map(|evt| match evt {
            Ok(XmlEvent::StartElement { name, attributes, .. }) => {
                let get_attribute = |attrib_name: &str| {
                    attributes.iter()
                        .find(|attr| attr.name.local_name == attrib_name)
                        .map(|attr| &attr.value)
                        .map(|v| Ok(v))
                        .unwrap_or_else(|| Err(ParseError::Custom(format!("{} requirement didn't have attribute: {}", &name, attrib_name).into())))
                };
                match name.local_name.as_str() {
                    "type" => Some(get_attribute("name").map(|n| Requirement::Type(n.clone()))),
                    "enum" => Some({
                        if let (Ok(name), Ok(extends)) = (get_attribute("name"), get_attribute("extends")) {
                            let offset_strategy = get_attribute("offset")
                                .and_then(|o| o.parse::<isize>().map_err(|_| ParseError::Custom("Offset didn't parse".into())))
                                .map(|offset| EnumExtensionStrategy::Offset {
                                    offset: offset,
                                    negated: get_attribute("dir").ok().map(|s| s.as_str()) == Some("-"),
                                });
                            let value_strategy = get_attribute("value").map(|v| EnumExtensionStrategy::Value(EnumValue::StrValue(v.clone())));
                            let bitpos_strategy = get_attribute("bitpos")
                                .and_then(|bp| bp.parse::<u8>().map_err(|_| ParseError::Custom("bitpos didn't parse".into())))
                                .map(|bp| EnumExtensionStrategy::Value(EnumValue::Bitpos(bp)));
                            offset_strategy.or(value_strategy).or(bitpos_strategy)
                                .map(|strategy| Requirement::EnumExtension {
                                    name: name.clone(),
                                    extends: extends.clone(),
                                    value: strategy,
                                })
                        } else {
                            get_attribute("name").and_then(|name| {
                                let value = {
                                    let value = get_attribute("value").map(|v| EnumValue::StrValue(v.clone())).ok();
                                    let bitpos = get_attribute("bitpos")
                                        .and_then(|bp| bp.parse::<u8>().map_err(|_| ParseError::Custom("bitpos didn't parse".into())))
                                        .map(|bp| EnumValue::Bitpos(bp))
                                        .ok();
                                    value.or(bitpos)
                                };
                                if let Some(value) = value {
                                    Ok(Requirement::Constant {
                                        name: name.clone(),
                                        value: value,
                                        ty: get_attribute("type").map(|s| s.clone()).ok(),
                                    })
                                } else {
                                    Ok(Requirement::Enum(name.clone()))
                                }
                            })
                        }
                    }),
                    "command" => Some(get_attribute("name").map(|n| Requirement::Command(n.clone()))),
                    _ => None,
                }
            },
            Err(e) => Some(Err(ParseError::Xml(e))),
            _ => None,
        }).next()
    }
}

#[derive(Debug, Clone)]
pub struct FeatureInfo {
    pub name: String,
    pub api: String,
    pub number: String,
    pub requirements: LinkedList<Requirement>,
}

impl FeatureInfo {
    pub fn parse_next_feature<It: Iterator<Item=XmlResult<XmlEvent>>>(mut events: It) -> Option<ParseResult<FeatureInfo>> {
        // This should have been implementable by tail call recursion
        let next_event = {
            let mut events = events.by_ref().skip_while(|evt| match evt {
                &Err(_) => false,
                &Ok(XmlEvent::StartElement { ref name, .. }) => name.local_name != "feature",
                &Ok(_) => true,
            });
            events.next()
        };
        next_event.map(|r| r.map_err(|e| ParseError::Xml(e)).and_then(|evt| match evt {
            XmlEvent::StartElement { name, attributes, .. } => {
                let node = xast::Node {
                    name: name.local_name,
                    attributes: attributes,
                    contents: LinkedList::new(),
                };
                let manditory_attribute = |name: &str| {
                    node.get_attribute(name)
                        .map(|s| Ok(String::from(s)))
                        .unwrap_or_else(|| Err(ParseError::Custom(format!("feature didn't have attribute: {}", name).into())))
                };
                let mut events = XmlContents::new_inside(events);
                let features = FromNextFn::new(|| Requirement::parse_next_requirement(&mut events))
                    .fold(Ok(LinkedList::new()), |l, f| {
                        let f = try!(f);
                        l.map(move |mut l| {
                            l.push_back(f);
                            l
                        })
                    });
                Ok(FeatureInfo {
                    name: try!(manditory_attribute("name")),
                    api: try!(manditory_attribute("api")),
                    number: try!(manditory_attribute("number")),
                    requirements: try!(features),
                })
            },
            _ => unreachable!(),
        }))
    }
}

#[derive(Debug, Clone)]
pub enum ExtensionType {
    Disabled,
    Device {
        supported: Regex,
    },
    Instance {
        supported: Regex,
    },
}

impl Default for ExtensionType {
    fn default() -> ExtensionType {
        ExtensionType::Disabled
    }
}

#[derive(Debug, Clone)]
pub struct ExtensionInfo {
    pub name: String,
    pub number: isize,
    pub author: Option<String>,
    pub contact: Option<String>,
    pub ty: ExtensionType,
    pub requires: LinkedList<String>, // optional. empty if not present
    pub protect: Option<String>,
    pub requirements: LinkedList<Requirement>,
}

#[derive(Debug)]
pub struct Registry {
    types: HashMap<String, TypeInfo>,
    features: LinkedList<FeatureInfo>,
    extensions: LinkedList<ExtensionInfo>,
}

impl Registry {
    pub fn new<It: Iterator<Item=XmlResult<XmlEvent>>, FeatureF: FnMut(&str) -> bool, ExtensionF: FnMut(&str) -> bool>(mut events: It, mut should_include_feature: FeatureF, mut should_include_extension: ExtensionF) -> ParseResult<Registry> {
        let mut types: Option<HashMap<String, TypeInfo>> = None;
        let mut features: LinkedList<FeatureInfo> = LinkedList::new();
        let mut extensions: LinkedList<ExtensionInfo> = LinkedList::new();

        {
            loop {
                let next_event = events.next();
                match next_event {
                    Some(Ok(XmlEvent::StartElement { name, attributes, .. })) => {
                        match name.local_name.as_str() {
                            "types" => {
                                let mut events = XmlContents::new_inside(&mut events);
                                let new_types = try!(FromNextFn::new(|| TypeInfo::parse_next_type(&mut events))
                                    .fold(Ok(HashMap::new()), |r, t| r.and_then(move |mut m| {
                                        let t = try!(t);
                                        m.insert(t.name().into(), t);
                                        Ok(m)
                                    })));
                                types = Some(new_types);
                            },
                            "features" => {
                                let mut events = XmlContents::new_inside(&mut events);
                                let new_features = try! {
                                    FromNextFn::new(|| FeatureInfo::parse_next_feature(&mut events))
                                        .fold(Ok(LinkedList::new()), |l, f| {
                                            let f = try!(f);
                                            l.map(move |mut l| {
                                                l.push_back(f);
                                                l
                                            })
                                        })
                                };
                                let mut append_features = |mut l: LinkedList<FeatureInfo>| {
                                    features.append(&mut l);
                                };
                                append_features(new_features);
                            },
                            _ => {},
                        }
                    },
                    Some(Ok(_)) => continue,
                    Some(Err(e)) => {
                        return Err(ParseError::Xml(e));
                    },
                    None => break, // If we have no more events, then we're done.
                }
            }
        }
        types
            .map(|t| Ok(t))
            .unwrap_or_else(|| Err(ParseError::Custom("No <types> tag was found in the registry".into())))
            .map(move |types| Registry {
                types: types,
                extensions: extensions,
                features: features,
            })
    }
}
