extern crate xml;

use std::{ borrow, env, fs, io, path };
use std::collections::{ HashMap, LinkedList };
use xml::reader::XmlEvent;
use xml::reader::Result as XmlResult;

mod xast;
mod xml_iter;

use xml_iter::{ XmlContents, XmlIteratorExtensions };

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
            let ret = self.events.next().map(|r| r.map(|e| match e {
                e @ XmlEvent::StartElement { .. } => {
                    self.depth = self.depth + 1;
                    e
                },
                e @ XmlEvent::EndElement { .. } => {
                    self.depth = self.depth - 1;
                    e
                },
                e => e,
            }));
            if self.depth < 1 {
                None
            } else {
                ret
            }
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

impl TypeInfo {
    fn name(&self) -> &str {
        use TypeInfo::*;
        match self {
            &Basetype(ref name, _) => name.as_str(),
            &Handle { ref name, .. } => name.as_str(),
            &Struct { ref name, .. } => name.as_str(),
        }
    }
}

#[derive(Debug, Clone)]
enum BitmaskValue {
    Value(String),
    BitIndex(u8),
}

#[derive(Debug, Clone)]
enum EnumInfo {
    Bitmask {
        name: String,
        values: LinkedList<(String, BitmaskValue)>,
    },
    Enum {
        name: String,
        values: LinkedList<(String, BitmaskValue)>,
    },
}

impl EnumInfo {
    fn name(&self) -> &str {
        use EnumInfo::*;
        match self {
            &Bitmask { ref name, .. } => name.as_str(),
            &Enum { ref name, .. } => name.as_str(),
        }
    }
}

#[derive(Debug, Clone)]
enum EnumExtensionStrategy {
    Offset(usize),
    Bitpos(u8),
}

#[derive(Debug, Clone)]
enum ExtensionRequirement {
    Command(String),
    Type(String),
    ReferenceEnum(String),
    Enum, // TODO: actually parse these properly
    EnumExtension {
        extends: String,
        strategy: EnumExtensionStrategy,
    },
}

impl ExtensionRequirement {
    fn read_next<It: Iterator<Item=XmlResult<XmlEvent>>>(events: It) -> Option<XmlResult<ExtensionRequirement>> {
        let mut events = XmlContents::new_inside(events).filter_map(|evt| match evt {
            Ok(XmlEvent::StartElement { name, attributes, .. }) => match name.local_name.as_str() {
                "command" =>  {
                    let name = attributes.into_iter().find(|attr| attr.name.local_name == "name").map(|attr| attr.value).unwrap();
                    Some(Ok(ExtensionRequirement::Command(name)))
                },
                "type" => {
                    let name = attributes.into_iter().find(|attr| attr.name.local_name == "name").map(|attr| attr.value).unwrap();
                    Some(Ok(ExtensionRequirement::Type(name)))
                },
                "enum" => {
                    if let Some(extends) = get_attribute("extends", attributes.iter()) {
                        let strategy = get_attribute("offset", attributes.iter()).and_then(|s| s.parse::<usize>().ok()).map(|o| EnumExtensionStrategy::Offset(o));
                        Some(Ok(ExtensionRequirement::EnumExtension {
                            extends: extends.into(),
                            strategy: strategy.unwrap(),
                        }))
                    } else if let (Some(value), Some(name)) = (get_attribute("value", attributes.iter()), get_attribute("name", attributes.iter())) {
                        Some(Ok(ExtensionRequirement::Enum))
                    } else {
                        let name = get_attribute("name", attributes.iter()).map(Into::into).unwrap();
                        Some(Ok(ExtensionRequirement::ReferenceEnum(name)))
                    }
                },
                _ => None,
            },
            _ => None
        });
        events.next()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExtensionType {
    Instance,
    Device,
    Feature,
}

#[derive(Debug, Clone)]
struct ExtensionInfo {
    pub name: String,
    pub extension_type: ExtensionType,
    pub dependencies: LinkedList<String>,
    pub requirements: LinkedList<ExtensionRequirement>,
    pub protect: Option<String>,
}

impl ExtensionInfo {
    /// Returns the name of the cargo feature required for this extension (if any)
    pub fn feature_name(&self) -> Option<&str> {
        match &self.protect {
            &Some(ref s) => {
                let s = s.as_str();
                if s.starts_with("VK_USE_PLATFORM_") {
                    Some(s.trim_left_matches("VK_USE_"))
                } else {
                    None
                }
            },
            &None => None
        }
    }

    pub fn parse_dependencies(dependencies: &str) -> LinkedList<String> {
        dependencies.split(",").map(Into::into).collect()
    }
}

#[derive(Debug)]
enum TopLevelElement {
    Types(Vec<TypeInfo>),
    Enums(Option<EnumInfo>),
    Extension(ExtensionInfo),
    BadExtension,
}

const TOP_LEVEL_NAMES: &'static [&'static str] = &[
    "types",
    "enums",
    "extension",
    "feature"
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

fn read_node<It: Iterator<Item=XmlResult<XmlEvent>>>(mut events: It, name: String, attributes: Vec<xml::attribute::OwnedAttribute>) -> XmlResult<xast::Node> {
    use std::collections::LinkedList;

    let mut events = Contents::new(&mut events);
    let mut node_stack: LinkedList<xast::Node> = LinkedList::new();
    node_stack.push_front(xast::Node {
        name: name,
        attributes: attributes,
        contents: Vec::new(),
    });
    for e in events {
        match try!(e) {
            XmlEvent::Characters(s) => {
                let node = node_stack.front_mut().unwrap();
                node.contents.push(xast::Content::Text(s));
            },
            XmlEvent::StartElement { name, attributes, .. } => {
                node_stack.push_front(xast::Node {
                    name: name.local_name,
                    attributes: attributes,
                    contents: Vec::new(),
                });
            },
            XmlEvent::EndElement { .. } => {
                let child = node_stack.pop_front().unwrap();
                let node = node_stack.front_mut().unwrap(); node.contents.push(xast::Content::Child(child)); },
            _ => {},
        }
    }
    Ok(node_stack.pop_front().unwrap())
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
        XmlEvent::StartElement { name, attributes, .. } => {
            read_node(&mut events, name.local_name, attributes).and_then(|node| {
                let mut name = node.get_child("name").map(xast::Node::concat_text).unwrap();
                let mut ty = node.get_child("type").unwrap().concat_text();
                ty = match ty.as_str() {
                    "void" => "c_void".into(),
                    "int" => "c_int".into(),
                    "float" => "c_float".into(),
                    _ => ty
                };
                let ty_extensions = node.concat_text();
                let ty_extensions = ty_extensions.trim();
                let prefix = if ty_extensions.starts_with("const") {
                    "const"
                } else {
                    "mut"
                };
                let ptr_lvl = ty_extensions.chars().filter(|&c| c == '*').count();
                if ptr_lvl > 0 {
                    let prefix = format!("*{} ", prefix);
                    let old_ty = ty;
                    ty = "".into();
                    for _ in 0..ptr_lvl {
                        ty.push_str(prefix.as_str());
                    }
                    ty.push_str(old_ty.as_str());
                }
                name = match name.as_str() {
                    "type" => "type_".into(),
                    _ => name,
                };
                Ok((name, ty))
            })
        },
        _ => unreachable!(),
    }))
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

struct EnumValues<It: Iterator<Item=XmlResult<XmlEvent>>> {
    events: XmlContents<It>,
}

impl<It: Iterator<Item=XmlResult<XmlEvent>>> EnumValues<It> {
    #[inline(always)]
    pub fn new(events: It) -> EnumValues<It> {
        EnumValues {
            events: XmlContents::new_inside(events),
        }
    }
}

impl<It: Iterator<Item=XmlResult<XmlEvent>>> Iterator for EnumValues<It> {
    type Item = XmlResult<(String, BitmaskValue)>;
    fn next(&mut self) -> Option<XmlResult<(String, BitmaskValue)>> {
        let next_event = {
            let mut events = self.events.by_ref().skip_while(|evt| match evt {
                &Err(_) => false,
                &Ok(XmlEvent::StartElement { ref name, .. }) if name.local_name == "enum" => false,
                &Ok(_) => true,
            });
            events.next()
        };
        next_event.map(|r| r.and_then(|evt| match evt {
            XmlEvent::StartElement { attributes, .. } => {
                let bitpos = get_attribute("bitpos", attributes.iter())
                    .and_then(|b| b.parse::<u8>().ok()).map(|bp| BitmaskValue::BitIndex(bp));
                let value = get_attribute("value", attributes.iter())
                    .map(|v| BitmaskValue::Value(v.into()));
                let name = get_attribute("name", attributes.iter()).unwrap();
                Ok((name.into(), bitpos.or(value).expect(format!("Enum {} had neither bitpos nor value", name).as_str())))
            },
            _ => unreachable!(),
        }))
    }
}

fn read_enums<It: Iterator<Item=XmlResult<XmlEvent>>>(mut events: It, attributes: Vec<xml::attribute::OwnedAttribute>) -> XmlResult<Option<EnumInfo>> {
    let ty = get_attribute("type", attributes.iter());
    let name: String = get_attribute("name", attributes.iter()).unwrap().into();
    let mut events = XmlContents::new_inside(events);
    let mut get_values = || -> XmlResult<LinkedList<(String, BitmaskValue)>> {
        let mut values: LinkedList<(String, BitmaskValue)> = LinkedList::new();
        for v in EnumValues::new(&mut events) {
            values.push_back(try!(v));
        }
        Ok(values)
    };
    match ty {
        Some("bitmask") => {
            Ok(Some(EnumInfo::Bitmask {
                name: name,
                values: try!(get_values()),
            }))
        },
        Some("enum") => {
            Ok(Some(EnumInfo::Enum {
                name: name,
                values: try!(get_values()),
            }))
        },
        Some(t) => Ok(None),
        None => Ok(None),
    }
}

fn read_extension<It: Iterator<Item=XmlResult<XmlEvent>>>(events: It, name: &str, attributes: Vec<xml::attribute::OwnedAttribute>) -> XmlResult<TopLevelElement> {
    let mut events = XmlContents::new_inside(events);
    let ty = if name == "feature" {
        ExtensionType::Feature
    } else {
        match get_attribute("type", attributes.iter()) {
            Some("device") => ExtensionType::Device,
            Some("instance") => ExtensionType::Instance,
            _ => {
                return Ok(TopLevelElement::BadExtension);
            },
        }
    };
    let dependencies = get_attribute("requires", attributes.iter()).map(ExtensionInfo::parse_dependencies);
    let protect: Option<String> = get_attribute("protect", attributes.iter()).map(|s| s.into());
    let name = attributes.into_iter().find(|attr| attr.name.local_name == "name").map(|attr| attr.value).unwrap();
    Ok(TopLevelElement::Extension(ExtensionInfo {
        name: name,
        extension_type: ty,
        dependencies: dependencies.unwrap_or(LinkedList::new()),
        requirements: FromNextFn::new(|| ExtensionRequirement::read_next(&mut events)).map(|r| r.unwrap()).collect(),
        protect: protect,
    }))
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
        XmlEvent::StartElement { name, attributes, .. } => match name.borrow().local_name {
            "types" => read_types(events),
            "enums" => read_enums(events, attributes).map(|v| TopLevelElement::Enums(v)),
            "extension" | "feature" => read_extension(events, name.local_name.as_str(), attributes),
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

struct ExtensionContext {
    types: HashMap<String, TypeInfo>,
    enums: HashMap<String, EnumInfo>,
}

impl ExtensionContext {
    pub fn update_with(&mut self, ext: &ExtensionInfo) {
        for req in ext.requirements.iter() {
            match req {
                &ExtensionRequirement::EnumExtension { ref extends, ref strategy } => {
                    if let Some(en) = enums.get_mut(extends) {
                        // TODO: left off here
                    }
                },
                _ => {},
            }
        }
    }
    pub fn write_extension<W: io::Write>(&mut self, mut out: W, extension: &ExtensionInfo) -> io::Result<()> {
    }
}

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

    let mut types: HashMap<String, TypeInfo> = HashMap::new();
    let mut enums: HashMap<String, EnumInfo> = HashMap::new();
    let mut extensions: LinkedList<ExtensionInfo> = LinkedList::new();
    for e in FromNextFn::new(|| read_top_level(&mut events)) {
        match e.unwrap() {
            TopLevelElement::Types(ts) => {
                for t in ts {
                    types.insert(t.name().into(), t);
                }
            },
            TopLevelElement::Enums(Some(info)) => {
                enums.insert(info.name().into(), info);
            },
            TopLevelElement::Enums(None) => {},
            TopLevelElement::Extension(info) => {
                extensions.push_back(info);
            },
            TopLevelElement::BadExtension => {},
        }
    }
    for e in extensions {
        let comment_type = if e.extension_type == ExtensionType::Feature {
            "feature"
        } else {
            "extension"
        };
        write!(&mut out_file, "// {}: {}\n", comment_type, &e.name).unwrap();
    }
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
            },
            TopLevelElement::Enums(None) => {},
            TopLevelElement::Enums(Some(EnumInfo::Enum { name, values })) => {
                let values = values.into_iter().map(|(name, v)| match v {
                    BitmaskValue::BitIndex(idx) => {
                        let value = 0b1 << idx;
                        (name, format!("0b{:b}", value))
                    },
                    BitmaskValue::Value(v) => {
                        (name, format!("{}", v))
                    },
                });
                write!(&mut out_file, "#[repr(C)]\n#[derive(Debug, Clone, Copy)]\npub enum {} {{\n", &name).unwrap();
                for (name, value) in values {
                    write!(&mut out_file, "    {} = {},\n", name, value).unwrap();
                }

                out_file.write_all(b"}\n").unwrap();
            },
            TopLevelElement::Enums(Some(EnumInfo::Bitmask { name, values })) => {
                // TODO: Read the `VkFlags` part from the associated <type> tag
                write!(&mut out_file, "smolder_ffi_bitmask! {{\n    {}, {},\n", &name, "VkFlags").unwrap();
                let values = values.into_iter().map(|(name, v)| match v {
                    BitmaskValue::BitIndex(idx) => {
                        let value = 0b1 << idx;
                        (name, format!("0b{:b}", value))
                    }
                    BitmaskValue::Value(v) => {
                        (name, format!("{}", v))
                    },
                });
                for (name, value) in values {
                    write!(&mut out_file, "    {}, {},\n", name, value).unwrap();
                }
                out_file.write_all(b"}\n").unwrap();

                let alias_name = name.as_str().replacen("FlagBits", "Flags", 1);
                write!(&mut out_file, "type {} = {};\n", &alias_name, &name).unwrap();
            },
            TopLevelElement::Extension(_) | TopLevelElement::BadExtension => {},
        }
    }
}
