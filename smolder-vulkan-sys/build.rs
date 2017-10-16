extern crate bindgen;
extern crate vulkan_api_registry;
extern crate xml;

use std::{ env, fs, fmt, io, path };
use std::collections::LinkedList;
use std::io::Write;
use path::Path;
use vulkan_api_registry::{ EnumsInfo, EnumsType, HandleType, ParseResult, Registry, TypeInfo, StructMember };
use xml::reader::EventReader;

fn is_struct_type<'a>(member: &'a StructMember) -> Option<&'a LinkedList<String>> {
    member.values.as_ref().and_then(|values| {
        let name_matches = member.name == "sType";
        let type_matches = member.ty.ty == "VkStructureType" && member.ty.constness.len() == 0;
        if name_matches && type_matches {
            Some(values)
        } else {
            None
        }
    })
}

fn is_next(member: &StructMember) -> bool {
    let name_matches = member.name == "pNext";
    let type_matches = member.ty.ty == "void" && member.ty.constness.len() == 1;
    name_matches && type_matches
}

struct PointerIndirection<'a, S: AsRef<str>> {
    last_member: Option<&'a StructMember>,
    constness: bool,
    length: S,
    first: bool,
}

impl<'a, S: AsRef<str>> PointerIndirection<'a, S> {
    #[inline]
    fn len(&self) -> &str {
        self.length.as_ref()
    }
    pub fn begin<W: io::Write>(&self, out: &mut W) -> io::Result<()> {
        if self.is_slice() {
            return out.write_all(b"VkSlice<'a, ");
        }
        if self.constness {
            write!(out, "&'a ")?;
        } else {
            write!(out, "&'a mut ")?;
        }
        if self.len() == "null-terminated" {
            out.write_all(b"NTV<")?;
        }
        Ok(())
    }

    pub fn end<W: io::Write>(&self, out: &mut W) -> io::Result<()> {
        if self.len() == "null-terminated" {
            out.write_all(b">")?;
        } else if self.is_slice() {
            write!(out, ", {}>", &self.last_member.unwrap().ty.ty)?;
        }
        Ok(())
    }

    pub fn is_slice(&self) -> bool {
        self.first && Some(self.len()) == self.last_member.map(|m| m.name.as_str())
    }
}

fn member_write<W: io::Write>(out: &mut W, member: &StructMember, last_member: Option<&StructMember>) -> Result<bool, io::Error> {
    write!(out, "{}: ", &member.name)?;
    if (member.ty.constness.len() <= 0) {
        write!(out, "{}", &member.ty.ty)?;
        Ok(false)
    } else {
        let lengths = member.len
            .as_ref()
            .map(|s| Box::new(s.as_str().split(",")) as Box<Iterator<Item=&str>>)
            .unwrap_or_else(|| Box::new(std::iter::repeat("1")));
        let mut first = true;
        let mut indirections: LinkedList<PointerIndirection<&str>> = member.ty.constness.iter().zip(lengths).map(|(&c, len)| {
            let ret = PointerIndirection {
                last_member: last_member,
                constness: c,
                length: len,
                first: first,
            };
            first = false;
            ret
        }).collect();
        let can_make_option = Some(true) == indirections.front().map(|ind| !ind.is_slice());
        if member.optional && can_make_option {
            out.write_all(b"Option<")?;
        }
        for indirection in &indirections {
            indirection.begin(out)?;
        }
        out.write_all(member.ty.ty.as_bytes())?;
        let should_skip_previous = indirections.front().map(|ind| ind.is_slice()).unwrap_or(false);
        while let Some(indirection) = indirections.pop_back() {
            indirection.end(out)?;
        }
        if member.optional && can_make_option {
            out.write_all(b">")?;
        }
        Ok(should_skip_previous)
    }
}

fn parse_registry<P: AsRef<Path>>(p: P) -> io::Result<ParseResult<Registry>> {
    let always_true = |_: &str| true;
    let events = try!(fs::File::open(p.as_ref()).map(|f| EventReader::new(f).into_iter()));
    Ok(Registry::new(events, &always_true, &always_true))
}

fn main() {
    println!("cargo:rustc-link-lib=vulkan");
    let registry = parse_registry("../Vulkan-Docs/src/spec/vk.xml").unwrap().unwrap();
    let bitmasks = registry.types.iter().filter_map(|(name, info)| match info {
        &TypeInfo::Bitmask { ref requires, ref ty, .. } => {
            if requires.len() == 1 {
                Some((name, ty, requires.front().unwrap()))
            } else {
                None
            }
        },
        _ => None,
    });
    let bitmask_enums = registry.enums.iter().filter_map(|&EnumsInfo { ref name, ty, .. }| {
        match ty {
            Some(EnumsType::Bitmask) => name.as_ref(),
            _ => None,
        }
    });
    let handles = registry.types.iter().filter_map(|(name, info)| match info {
        &TypeInfo::Handle { ref parent, ty, .. } => Some((name, parent.as_ref(), ty)),
        _ => None,
    });
    let extendable_structs = registry.types.iter().filter_map(|(name, info)| match info {
        &TypeInfo::Struct { ref name, ref members, .. } => {
            let mut members = members.iter();
            let s_type = members.next().and_then(is_struct_type);
            let snd = members.next();
            match (s_type, snd) {
                (Some(s_type), Some(snd)) if is_next(snd) => {
                    Some((name, s_type, members))
                },
                _ => None
            }
        },
        _ => None,
    });
    let bindings = {
        let mut bindings = bindgen::Builder::default()
            .header("wrapper.h")
            .ignore_functions()
            .prepend_enum_name(false); // We don't want to add enum name in front of bitmask constants since vulkan already does that for us
        bindings = bitmask_enums.fold(bindings, |bindings, name| {
            bindings
                .bitfield_enum(name)
        });
        bindings = bitmasks.fold(bindings, |bindings, (name, ty, flag_type)| {
            bindings
                .hide_type(name)
                .raw_line(format!("vk_flags!({}, {}, {});", name, ty, flag_type))
        });
        bindings = handles.fold(bindings, |mut bindings, (name, _, ty)| {
            let underlying = format!("{}_T", name);
            if ty == HandleType::NonDispatchable {
                bindings = bindings
                    .hide_type(name)
                    .raw_line(format!("vk_non_dispatchable_handle!({}, {});", name, &underlying));
            }
            bindings.opaque_type(&underlying)
        });
        bindings.generate().unwrap()
    };
    let out_path = env::var("OUT_DIR").map(|ref s| {
        let s: &Path = s.as_ref();
        s.join("bindings.rs")
    }).expect("OUT_DIR should be set");
    bindings.write_to_file(out_path).unwrap();
    {
        let mut safe_out_file = env::var("OUT_DIR").map(|ref s| {
            let s: &Path = s.as_ref();
            s.join("safe_structs.rs")
        }).map(|p| fs::File::create(p).unwrap()).expect("OUT_DIR should be set");
        for (name, s_type, members) in extendable_structs {
            safe_out_file.write_all(b"vk_extendable_struct! {\n").unwrap();
            write!(&mut safe_out_file, "pub struct ({}, {}Base) -> ({}) {{\n", name, name, s_type.front().unwrap()).unwrap();
            let mut last_member = None;
            let mut members = members.peekable();
            let mut s: Option<Vec<u8>> = None;
            loop {
                let next_value = members.next();
                if let Some(member) = next_value {
                    let mut news: Vec<u8> = Vec::new();
                    if !member_write(&mut news, member, last_member).unwrap() {
                        if let Some(ref to_write) = s {
                            safe_out_file.write_all(to_write.as_slice()).unwrap();
                            safe_out_file.write_all(b",\n").unwrap();
                        }
                    }
                    last_member = Some(member);
                    s = Some(news);
                } else {
                    break;
                }
            }
            safe_out_file.write_all(b"}\n}\n").unwrap();
        }
    }
}
