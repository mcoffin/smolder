#[macro_use] extern crate lazy_static;
extern crate libc;
extern crate regex;
extern crate xml;

mod iter_util;
mod xast;
mod xml_iter;
pub mod vk_platform;

use iter_util::FromNextFn;
use regex::Regex;
use std::borrow::Cow;
use std::collections::{ BTreeSet, HashMap, LinkedList };
use std::iter::FromIterator;
use std::{ fmt };
use xml::attribute::OwnedAttribute;
use xml::reader::XmlEvent;
use xml::reader::Result as XmlResult;
use xml_iter::XmlContents;

#[derive(Debug)]
pub enum ParseError {
    UnexpectedEOF,
    Custom(Cow<'static, str>),
    Regex(regex::Error),
    Xml(xml::reader::Error),
    Impossible,
}

pub type ParseResult<T> = Result<T, ParseError>;

pub trait XmlParse: Sized {
    fn parse<It: Iterator<Item=XmlResult<XmlEvent>>>(events: It, name: String, attributes: Vec<OwnedAttribute>) -> ParseResult<Self>;
}

#[derive(Clone)]
pub struct TyperefInfo {
    pub ty: String,
    pub constness: Vec<bool>,
}

fn parse_constness<C: FromIterator<bool>>(ptr_info: &str) -> C {
    lazy_static! {
        static ref TYPEREF_PTR_PATTERN: Regex = Regex::new(r"\s*(const)?\s*\*").unwrap();
    }
    TYPEREF_PTR_PATTERN.captures_iter(ptr_info).map(|caps| {
        caps.get(1)
            .map(|s| s.as_str().eq("const"))
            .unwrap_or(false)
    }).collect()
}

impl TyperefInfo {
    pub fn parse_node(node: &xast::Node) -> ParseResult<TyperefInfo> {
        let ty = try! {
            node.get_attribute_or_child("type")
                .map(|s| Ok(s))
                .unwrap_or(Err(ParseError::Custom("typeref didn't have a type".into())))
        };
        let ptr_info = node.concat_text();
        lazy_static! {
            static ref TYPEREF_PTR_PATTERN: Regex = Regex::new(r"\s*(const)?\s*\*").unwrap();
        }
        let ty_constness: Vec<bool> = parse_constness(node.concat_text().as_str());
        Ok(TyperefInfo {
            ty: ty.into(),
            constness: ty_constness,
        })
    }
}

impl fmt::Display for TyperefInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        for &level in self.constness.iter() {
            let s = if level {
                "*const "
            } else {
                "*mut "
            };
            try!(f.write_str(s));
        }
        write!(f, "{}", self.ty)
    }
}

impl fmt::Debug for TyperefInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "\"{}\"", self)
    }
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

impl StructMember {
    pub fn parse_node(node: &xast::Node) -> ParseResult<StructMember> {
        let name = node.get_attribute_or_child("name")
            .map(|n| Ok(n))
            .unwrap_or(Err(ParseError::Custom("member did not have a name".into())));
        let values: Option<LinkedList<String>> = node.get_attribute("values")
            .map(|s| s.split(",").map(Into::into).collect());
        let externsync = node.get_attribute("externsync")
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false);
        let optional = node.get_attribute("optional")
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false);
        let noautovalidity = node.get_attribute("noautovalidity")
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false);
        let ty = TyperefInfo::parse_node(&node);
        Ok(StructMember {
            name: try!(name.map(Into::into)),
            ty: try!(ty),
            values: values,
            len: node.get_attribute("len").map(Into::into),
            altlen: node.get_attribute("altlen").map(Into::into),
            externsync: externsync,
            optional: optional,
            noautovalidity: noautovalidity,
        })
    }
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
        // TODO: bring these back
        //arguments: LinkedList<(String, TyperefInfo)>,
        //return_type: TyperefInfo,
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
    Union {
        name: String,
        members: LinkedList<StructMember>,
    },
    Include(String),
    Uncategorized {
        name: String,
        node: xast::Node,
    },
}

fn parse_members(node: &xast::Node) -> ParseResult<LinkedList<StructMember>> {
    node.contents.iter().filter_map(|c| match c {
        &xast::Content::Child(ref child) => {
            if child.name == "member" {
                Some(child)
            } else {
                None
            }
        },
        _ => None
    }).map(StructMember::parse_node).fold(Ok(LinkedList::new()), |l, m| l.and_then(move |mut l| {
        let m = try!(m);
        l.push_back(m);
        Ok(l)
    }))
}

fn parse_funcpointer(node: &xast::Node) -> ParseResult<TypeInfo> {
    let name = try! {
        node.get_attribute_or_child("name")
            .map(|s| Ok(s))
            .unwrap_or(Err(ParseError::Custom("funcpointer did not have a name".into())))
    };
    unimplemented!()
}

impl TypeInfo {
    pub fn name(&self) -> &str {
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
            &Union { ref name, .. } => name,
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
                    node.get_attribute_or_child("type")
                        .map(|ty| Ok(ty.into()))
                        .unwrap_or(Err(ParseError::Custom("basetype did not have a type tag".into())))
                },
            },
            Some("bitmask") => TypeInfo::Bitmask {
                name: name.into(),
                ty: try! {
                    node.get_attribute_or_child("type")
                        .map(|ty| Ok(ty.into()))
                        .unwrap_or(Err(ParseError::Custom("bitmask did not have a type tag".into())))
                },
            },
            Some("define") => TypeInfo::Define(name.into()),
            Some("enum") => TypeInfo::Enum {
                name: name.into(),
                values: LinkedList::new()
            },
            Some("struct") => {
                let members = parse_members(&node);
                TypeInfo::Struct {
                    name: name.into(),
                    members: try!(members),
                    extends: LinkedList::new(),
                }
            },
            Some("union") => {
                let members = parse_members(&node);
                TypeInfo::Union {
                    name: name.into(),
                    members: try!(members),
                }
            },
            Some("funcpointer") => try!(parse_funcpointer(&node)),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

impl ParameterInfo {
    pub fn parse_node(node: &xast::Node) -> ParseResult<ParameterInfo> {
        let name = node.get_attribute_or_child("name")
            .map(|s| Ok(s))
            .unwrap_or(Err(ParseError::Custom("parameter didn't have name".into())));
        let ty = TyperefInfo::parse_node(node);
        let optional = node.get_attribute("optional")
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false);
        Ok(ParameterInfo {
            name: try!(name).into(),
            ty: try!(ty),
            optional: optional,
        })
    }
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

impl CommandInfo {
    pub fn parse_next_command<It: Iterator<Item=XmlResult<XmlEvent>>>(mut events: It) -> Option<ParseResult<CommandInfo>> {
        // TODO: this should be implementable with tailcall recursion
        let next_event = {
            let mut events = events.by_ref().skip_while(|r| r.as_ref().map(|evt| match evt {
                &XmlEvent::StartElement { ref name, .. } if name.local_name == "command" => false,
                _ => true,
            }).unwrap_or(false));
            events.next()
        };
        next_event.map(|r| r.map_err(|e| ParseError::Xml(e)).and_then(|evt| match evt {
            XmlEvent::StartElement { name, attributes, .. } => {
                let events = XmlContents::new_inside(events);
                <Self as XmlParse>::parse(events, name.local_name, attributes)
            },
            _ => unreachable!(),
        }))
    }
}

fn csv_attribute<C: FromIterator<String> + Default>(node: &xast::Node, attribute: &str) -> C {
    node.get_attribute(attribute)
        .map(|s| s.split(",").map(Into::into).collect())
        .unwrap_or_else(Default::default)
}

impl XmlParse for CommandInfo {
    fn parse<It: Iterator<Item=XmlResult<XmlEvent>>>(events: It, name: String, attributes: Vec<OwnedAttribute>) -> ParseResult<CommandInfo> {
        let node: xast::Node = try!(XmlParse::parse(events, name, attributes));
        let proto = try! {
            node.get_child("proto")
                .map(|n| Ok(n))
                .unwrap_or(Err(ParseError::Custom("command did not have a prototype".into())))
        };
        let return_type: TyperefInfo = try!(TyperefInfo::parse_node(proto));
        let name = try! {
            proto.get_attribute_or_child("name")
                .map(|s| Ok(s))
                .unwrap_or(Err(ParseError::Custom("command did not have a name".into())))
        };
        let params = node.contents.iter().filter_map(|e| match e {
            &xast::Content::Child(ref c) if c.name == "param" => {
                Some(c)
            },
            _ => None,
        }).map(ParameterInfo::parse_node).fold(Ok(LinkedList::new()), |l, p| l.and_then(move |mut l| {
            let p = try!(p);
            l.push_back(p);
            Ok(l)
        }));
        let queues = csv_attribute(&node, "queues");
        let successcodes = csv_attribute(&node, "successcodes");
        let errorcodes = csv_attribute(&node, "errorcodes");
        let renderpass = match node.get_attribute("renderpass") {
            Some("outside") => Some(CommandRenderPassInfo::Outside),
            Some("inside") => Some(CommandRenderPassInfo::Inside),
            Some("both") => Some(CommandRenderPassInfo::Both),
            Some(s) => {
                return Err(ParseError::Custom(format!("bad renderpass attribute: {}", s).into()));
            },
            None => None
        };
        let cmdbufferlevel = match node.get_attribute("cmdbufferlevel") {
            Some(s) => {
                let mut set = BTreeSet::new();
                for s in s.split(",") {
                    match s {
                        "primary" => {
                            set.insert(CommandBufferLevel::Primary);
                        },
                        "secondary" => {
                            set.insert(CommandBufferLevel::Secondary);
                        },
                        s => {
                            return Err(ParseError::Custom(format!("bad cmdbufferlevel attribute: {}", s).into()));
                        },
                    }
                }
                Some(set)
            },
            None => None,
        };
        let pipeline = match node.get_attribute("pipeline") {
            Some("compute") => Some(CommandPipelineType::Compute),
            Some("transfer") => Some(CommandPipelineType::Transfer),
            Some("graphics") => Some(CommandPipelineType::Graphics),
            Some(s) => {
                return Err(ParseError::Custom(format!("bad pipeline attribute: {}", s).into()));
            },
            None => None,
        };
        Ok(CommandInfo {
            name: name.into(),
            queues: queues,
            successcodes: successcodes,
            errorcodes: errorcodes,
            renderpass: renderpass,
            cmdbufferlevel: cmdbufferlevel,
            pipeline: pipeline,
            return_type: return_type,
            params: try!(params),
        })
    }
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

impl ExtensionInfo {
    pub fn parse_next_extension<It: Iterator<Item=XmlResult<XmlEvent>>>(mut events: It) -> Option<ParseResult<ExtensionInfo>> {
        // TODO: this should be implementable with tail-call recursion
        let next_event = {
            let mut events = events.by_ref().skip_while(|evt| match evt {
                &Err(_) => false,
                &Ok(XmlEvent::StartElement { ref name, .. }) if name.local_name == "extension" => false,
                &Ok(_) => true,
            });
            events.next()
        };
        next_event.map(|r| r.map_err(|e| ParseError::Xml(e)).and_then(|evt| match evt {
            XmlEvent::StartElement { name, attributes, .. } => {
                let mut events = XmlContents::new_inside(&mut events);
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
                let number = node.get_attribute("number")
                    .map(|n| Ok(n))
                    .unwrap_or(Err(ParseError::Custom("no extension number found".into())))
                    .and_then(|n| n.parse::<isize>().map_err(|_| ParseError::Custom("extension number couldn't be parsed".into())));
                let ty: ParseResult<ExtensionType> = node.get_attribute("type").map(|ty| {
                    manditory_attribute("supported")
                        .and_then(|ref pat| Regex::new(pat.as_str()).map_err(|e| ParseError::Regex(e)))
                        .and_then(|supported| match ty {
                            "instance" => Ok(ExtensionType::Instance {
                                supported: supported,
                            }),
                            "device" => Ok(ExtensionType::Device {
                                supported: supported,
                            }),
                            t => Err(ParseError::Custom(format!("Unknown extension type: {}", t).into())),
                        })
                }).unwrap_or(Ok(Default::default()));
                let requires: LinkedList<String> = node.get_attribute("requires")
                    .map(|s| s.split(",").map(Into::into).collect())
                    .unwrap_or_else(|| LinkedList::new());
                let requirements = FromNextFn::new(|| Requirement::parse_next_requirement(&mut events))
                    .fold(Ok(LinkedList::new()), |l, r| l.and_then(move |mut l| {
                        let r = try!(r);
                        l.push_back(r);
                        Ok(l)
                    }));
                Ok(ExtensionInfo {
                    name: try!(manditory_attribute("name")),
                    number: try!(number),
                    author: manditory_attribute("author").ok(),
                    contact: manditory_attribute("contact").ok(),
                    ty: try!(ty),
                    requires: requires,
                    protect: manditory_attribute("protect").ok(),
                    requirements: try!(requirements),
                })
            },
            _ => unreachable!(),
        }))
    }
}

#[derive(Debug)]
pub struct Registry {
    pub types: HashMap<String, TypeInfo>,
    pub commands: HashMap<String, CommandInfo>,
    pub features: LinkedList<FeatureInfo>,
    pub extensions: LinkedList<ExtensionInfo>,
}

impl Registry {
    pub fn new<It: Iterator<Item=XmlResult<XmlEvent>>, FeatureF: FnMut(&str) -> bool, ExtensionF: FnMut(&str) -> bool>(mut events: It, mut should_include_feature: FeatureF, mut should_include_extension: ExtensionF) -> ParseResult<Registry> {
        let mut types: Option<HashMap<String, TypeInfo>> = None;
        let mut commands: HashMap<String, CommandInfo> = HashMap::new();
        let mut features: LinkedList<FeatureInfo> = LinkedList::new();
        let mut extensions: LinkedList<ExtensionInfo> = LinkedList::new();

        {
            loop {
                let next_event = events.next();
                match next_event {
                    Some(Ok(XmlEvent::StartElement { name, attributes, namespace })) => {
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
                            "feature" => {
                                let start_event = XmlEvent::StartElement {
                                    name: name,
                                    attributes: attributes,
                                    namespace: namespace,
                                };
                                let mut events = std::iter::once(Ok(start_event))
                                    .chain(&mut events);
                                let feature = FeatureInfo::parse_next_feature(&mut events)
                                    .unwrap_or(Err(ParseError::Impossible));
                                features.push_back(try!(feature));
                            },
                            "extensions" => {
                                let mut events = XmlContents::new_inside(&mut events);
                                let new_extensions = try! {
                                    FromNextFn::new(|| ExtensionInfo::parse_next_extension(&mut events))
                                        .fold(Ok(LinkedList::new()), |l, e| l.and_then(move |mut l| {
                                            let e = try!(e);
                                            l.push_back(e);
                                            Ok(l)
                                        }))
                                };
                                let mut append_extensions = |mut l: LinkedList<ExtensionInfo>| {
                                    extensions.append(&mut l);
                                };
                                append_extensions(new_extensions);
                            },
                            "commands" => {
                                let mut events = XmlContents::new_inside(&mut events);
                                for new_command in FromNextFn::new(|| CommandInfo::parse_next_command(&mut events)) {
                                    let cmd = try!(new_command);
                                    commands.insert(cmd.name.clone(), cmd);
                                }
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
                commands: commands,
                extensions: extensions,
                features: features,
            })
    }
}
