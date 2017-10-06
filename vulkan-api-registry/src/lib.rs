extern crate regex;
extern crate xml;

use std::collections::{ BTreeSet, LinkedList };
use regex::Regex;
use xml::reader::XmlEvent;
use xml::reader::Result as XmlResult;

mod xml_iter;
use xml_iter::XmlContents;

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
        values: LinkedList<(String, EnumValue)>,
    },
    Define(String), // TODO: unused rn for rust
    Enum {
        name: String,
        values: LinkedList<(String, EnumValue)>,
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
    Enum(String),
    Command(String),
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

#[derive(Debug, Clone)]
pub struct FeatureInfo {
    pub name: String,
    pub api: String,
    pub number: String,
    pub requirements: LinkedList<Requirement>,
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

pub struct Registry {
}
