extern crate bindgen;
extern crate vulkan_api_registry;
extern crate xml;

use std::{ env, fs, fmt, io, path };
use std::borrow::Cow;
use std::collections::{ BTreeSet, LinkedList };
use std::io::Write;
use path::Path;
use vulkan_api_registry::{ EnumsInfo, EnumsType, HandleType, ParseResult, Registry, TypeInfo, StructMember };
use xml::reader::EventReader;

//fn is_struct_type<'a>(member: &'a StructMember) -> Option<&'a LinkedList<String>> {
//    member.values.as_ref().and_then(|values| {
//        let name_matches = member.name == "sType";
//        let type_matches = member.ty.ty == "VkStructureType" && member.ty.constness.len() == 0;
//        if name_matches && type_matches {
//            Some(values)
//        } else {
//            None
//        }
//    })
//}
//
//fn is_next(member: &StructMember) -> bool {
//    let name_matches = member.name == "pNext";
//    let type_matches = member.ty.ty == "void" && member.ty.constness.len() == 1;
//    name_matches && type_matches
//}

fn parse_registry<P: AsRef<Path>>(p: P) -> io::Result<ParseResult<Registry>> {
    let always_true = |_: &str| true;
    let events = try!(fs::File::open(p.as_ref()).map(|f| EventReader::new(f).into_iter()));
    Ok(Registry::new(events, &always_true, &always_true))
}

//#[derive(Debug, Clone, Copy)]
//enum PointerType<M: AsRef<str>> {
//    Reference,
//    NullTerminated,
//    MemberSized(M),
//    LatexSized,
//}
//
//impl<M: AsRef<str>> PointerType<M> {
//    fn is_pub_safe(&self) -> bool {
//        use PointerType::*;
//        match self {
//            &Reference => true,
//            &NullTerminated => true,
//            &_ => false,
//        }
//    }
//}
//
//impl<M: AsRef<str>> Default for PointerType<M> {
//    #[inline]
//    fn default() -> PointerType<M> {
//        PointerType::Reference
//    }
//}
//
//struct PointerInfo<T: AsRef<str>> {
//    constness: bool,
//    ty: PointerType<T>,
//}
//
//impl<T: AsRef<str>> PointerInfo<T> {
//    fn fmt_begin<W: io::Write>(&self, mut w: W) -> io::Result<()> {
//        match (&self.ty, self.constness) {
//            (&PointerType::NullTerminated, c) => {
//                let addon = if c {
//                    ""
//                } else {
//                    "mut "
//                };
//                write!(&mut w, "&'a {}NTV<", addon)
//            },
//            (_, true) => {
//                w.write_all(b"&'a ")
//            },
//            (_, false) => {
//                w.write_all(b"&'a mut ")
//            },
//        }
//    }
//    fn fmt_end<W: io::Write>(&self, mut w: W) -> io::Result<()> {
//        match &self.ty {
//            &PointerType::NullTerminated => {
//                w.write_all(b">")
//            },
//            _ => Ok(())
//        }
//    }
//}
//
//fn all_pub_safe<'a, It: Iterator<Item=&'a PointerInfo<&'a str>>>(pointer_info: It) -> bool {
//    pointer_info.map(|info| info.ty).fold(true, |last, t| last && t.is_pub_safe())
//}
//
//trait MemberExt {
//    fn pointer_info<'a>(&'a self) -> LinkedList<PointerInfo<&'a str>>;
//}
//
//impl MemberExt for StructMember {
//    fn pointer_info<'a>(&'a self) -> LinkedList<PointerInfo<&'a str>> {
//        let constness = self.ty.constness.iter().map(|&c| c);
//        let length = self.len
//            .as_ref()
//            .map(|s| s.as_str().split(","))
//            .map(|it| Box::new(it) as Box<Iterator<Item=&str>>)
//            .unwrap_or_else(|| Box::new(std::iter::repeat("1")))
//            .chain(std::iter::repeat("1"));
//        constness.zip(length).map(|(c, l)| {
//            let ty = match l {
//                "null-terminated" => PointerType::NullTerminated,
//                "1" => PointerType::Reference,
//                s => {
//                    if s.starts_with("latexmath:") {
//                        PointerType::LatexSized
//                    } else {
//                        PointerType::MemberSized(s)
//                    }
//                }
//            };
//            PointerInfo {
//                constness: c,
//                ty: ty,
//            }
//        }).collect()
//    }
//}

struct StructInfo<'a> {
    name: &'a str,
    members: &'a LinkedList<StructMember>,
    extends: &'a LinkedList<String>,
    registry: &'a Registry,
}

impl<'a> StructInfo<'a> {
    fn base_name(&self) -> String {
        if self.is_extendable() {
            format!("{}Base", self.name)
        } else {
            String::from(self.name)
        }
    }

    #[inline]
    fn is_extendable(&self) -> bool {
        self.extendable().is_some()
    }

    fn extendable(&self) -> Option<&str> {
        let mut members = self.members.into_iter();
        let s_type = members.next()
            .and_then(|fst| fst.values.as_ref())
            .and_then(|v| v.front());
        let snd = members.next().and_then(|snd| if snd.name == "pNext" && snd.ty.ty == "void" {
            Some(snd)
        } else {
            None
        });
        match (s_type, snd) {
            (maybe_s_type, Some(_)) => {
                maybe_s_type.map(|s| s.as_str())
            },
            _ => None,
        }
    }

    fn used_as_count(&self, member_name: &str) -> bool {
        self.members.into_iter()
            .filter_map(|member| member.len.as_ref().map(|len| len.split(",")))
            .filter_map(|mut lens| lens.find(|&len| len == member_name))
            .next()
            .is_some()
    }

    fn base_members(&self) -> std::iter::Skip<std::collections::linked_list::Iter<StructMember>> {
        if self.is_extendable() {
            self.members.into_iter().skip(2)
        } else {
            self.members.into_iter().skip(0)
        }
    }

    fn type_params(&self) -> BTreeSet<&'a str> {
        let lifetimes = self.base_members().into_iter().flat_map(|member| if member.ty.constness.len() > 0 {
            Some("'a")
        } else {
            None
        });
        let handle_lifetimes = self.base_members().into_iter().flat_map(|member| match self.registry.types.get(member.name.as_str()) {
            Some(&TypeInfo::Handle { .. }) => {
                let mut single = BTreeSet::new();
                single.insert("'a");
                single
            },
            Some(&TypeInfo::Struct { ref name, ref members, ref extends }) => {
                let info = StructInfo {
                    name: name,
                    members: members,
                    extends: extends,
                    registry: self.registry
                };
                info.type_params()
            },
            o => {
                println!("other type: {:?}", &o);
                BTreeSet::new()
            },
        });
        lifetimes.chain(handle_lifetimes).collect()
    }
}

enum PointerType<'a> {
    Reference,
    NullTerminated,
    MemberSized(&'a str),
    LatexSized(&'a str),
}

struct PointerInfo<'a> {
    constness: bool,
    ty: PointerType<'a>,
}

impl<'a> PointerInfo<'a> {
    fn begin<W: fmt::Write>(&self, f: &mut W) -> fmt::Result {
        f.write_str("&'a ")?;
        if !self.constness {
            f.write_str("mut ")?;
        }
        match &self.ty {
            &PointerType::NullTerminated => {
                f.write_str("NTV<")?;
            },
            _ => {},
        }
        Ok(())
    }

    fn end<W: fmt::Write>(&self, f: &mut W) -> fmt::Result {
        match &self.ty {
            &PointerType::NullTerminated => {
                f.write_str(">")?;
            },
            _ => {},
        }
        Ok(())
    }
}

trait MemberExt {
    fn clean_name(&self) -> &str;
    fn pointer_info<'a>(&'a self) -> Vec<PointerInfo<'a>>;

    fn is_slice(&self) -> bool {
        let ptr_infos = self.pointer_info();
        ptr_infos.iter().find(|info| match &info.ty {
            &PointerType::MemberSized(_) => true,
            _ => false,
        }).is_some()
    }
}

impl MemberExt for StructMember {
    fn clean_name(&self) -> &str {
        match self.name.as_str() {
            "type" => "ty",
            n => n,
        }
    }

    fn pointer_info<'a>(&'a self) -> Vec<PointerInfo<'a>> {
        let constness = self.ty.constness.iter().map(|&c| c);
        let len = self.len.as_ref()
            .map(|s| s.as_str().split(","))
            .map(|it| Box::new(it) as Box<Iterator<Item=&str>>)
            .unwrap_or_else(|| Box::new(std::iter::repeat("1")))
            .chain(std::iter::repeat("1"))
            .map(|l| match l {
                "1" => PointerType::Reference,
                "null-terminated" => PointerType::NullTerminated,
                s => {
                    if s.starts_with("latexmath:") {
                        PointerType::LatexSized(s)
                    } else {
                        PointerType::MemberSized(s)
                    }
                },
            });
        constness.zip(len).map(|(c, ty)| PointerInfo {
            constness: c,
            ty: ty,
        }).collect()
    }
}

trait RegistryExt {
    fn struct_member_representation_of<'a>(&'a self, ty: &'a str) -> Cow<'a, str>;
}

impl RegistryExt for Registry {
    fn struct_member_representation_of<'a>(&'a self, ty: &'a str) -> Cow<'a, str> {
        self.types.get(ty).map(|info| match info {
            &TypeInfo::Handle { ref name, .. } => format!("{}<'a>", name).into(),
            _ => ty.into(),
        }).unwrap_or(ty.into())
    }
}

fn clean_c_name<'a>(name: &'a str) -> &'a str {
    match name {
        "float" => "c_float",
        "long" => "c_long",
        "int" => "c_int",
        "void" => "c_void",
        s => s,
    }
}

fn print_comma_sep_iter<'a, W: fmt::Write, It: Iterator<Item=&'a T>, T: 'a + fmt::Display>(w: &mut W, items: It) -> fmt::Result {
    let mut items = items.peekable();
    loop {
        let next_item = items.next();
        match next_item {
            Some(item) => {
                if items.peek().is_some() {
                    write!(w, "{},", item)?;
                } else {
                    write!(w, "{}", item)?;
                }
            },
            None => break,
        }
    }
    Ok(())
}

impl<'a> fmt::Display for StructInfo<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::borrow::Borrow;
        let base_name = self.base_name();
        write!(f, "#[repr(C)]\n#[derive(Clone, Copy)]\npub struct {}", &base_name)?;
        let t_params = self.type_params();
        if t_params.len() > 0 {
            f.write_str("<")?;
            print_comma_sep_iter(f, t_params.iter())?;
            f.write_str(">")?;
        }
        f.write_str(" {\n")?;
        let members_iter = self.members.into_iter();
        let members_iter = if self.is_extendable() {
            members_iter.skip(2)
        } else {
            members_iter.skip(0)
        };
        for member in members_iter {
            f.write_str("    ")?;
            let is_private = self.used_as_count(member.name.as_str()) || member.is_slice();
            if !is_private {
                f.write_str("pub ")?;
            }
            write!(f, "{}: ", member.clean_name())?;
            let mut ptr_infos = member.pointer_info();
            for info in &ptr_infos {
                info.begin(f)?;
            }
            f.write_str(self.registry.struct_member_representation_of(clean_c_name(member.ty.ty.as_str())).borrow())?;
            ptr_infos.reverse();
            for info in &ptr_infos {
                info.end(f)?;
            }
            f.write_str(",\n")?;
        }
        f.write_str("}\n")?;
        match self.extendable() {
            Some(s_type) => {
                use std::fmt::Write;
                write!(f, "vk_extendable_struct!({}", &base_name)?;
                if t_params.len() > 0 {
                    f.write_char('(')?;
                    print_comma_sep_iter(f, t_params.iter())?;
                    f.write_char(')')?;
                }
                write!(f, ", ({}, {}));\n", self.name, s_type)?;
            },
            None => {},
        }
        Ok(())
    }
}

fn write_structs<P: AsRef<Path>>(registry: &Registry, p: P) {
    let mut out_file = fs::File::create(p).unwrap();
    let structs = registry.types.iter().filter_map(|(name, info)| match info {
        &TypeInfo::Struct { ref name, ref members, ref extends } => Some(StructInfo {
            name: name.as_str(),
            members: members,
            extends: extends,
            registry: registry,
        }),
        _ => None,
    });
    for info in structs {
        write!(&mut out_file, "{}", &info).unwrap();
    }
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
    let structs = registry.types.iter().filter_map(|(name, info)| match info {
        &TypeInfo::Struct { .. } => Some(name),
        _ => None,
    });
    //let extendable_structs = registry.types.iter().filter_map(|(name, info)| match info {
    //    &TypeInfo::Struct { ref name, ref members, .. } => {
    //        let mut members = members.iter();
    //        let s_type = members.next().and_then(is_struct_type);
    //        let snd = members.next();
    //        match (s_type, snd) {
    //            (Some(s_type), Some(snd)) if is_next(snd) => {
    //                Some((name, s_type, members))
    //            },
    //            _ => None
    //        }
    //    },
    //    _ => None,
    //});
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
            } else {
                bindings = bindings
                    .hide_type(name)
                    .raw_line(format!("vk_handle!({}, {});", name, &underlying));
            }
            bindings.opaque_type(&underlying)
        });
        bindings = structs.fold(bindings, |mut bindings, name| {
            bindings
                .hide_type(name)
        });
        bindings.generate().unwrap()
    };
    let out_path = env::var("OUT_DIR").map(|ref s| {
        let s: &Path = s.as_ref();
        s.join("bindings.rs")
    }).expect("OUT_DIR should be set");
    bindings.write_to_file(out_path).unwrap();
    let structs_out_path = env::var("OUT_DIR").map(|ref s| {
        let s: &Path = s.as_ref();
        s.join("safe_structs.rs")
    }).expect("OUT_DIR should be set");
    write_structs(&registry, structs_out_path);
    //{
    //    let mut safe_out_file = env::var("OUT_DIR").map(|ref s| {
    //        let s: &Path = s.as_ref();
    //        s.join("safe_structs.rs")
    //    }).map(|p| fs::File::create(p).unwrap()).expect("OUT_DIR should be set");
    //    for (name, s_type, members) in extendable_structs {
    //        let members: LinkedList<&StructMember> = members.collect();
    //        write!(&mut safe_out_file, "pub struct {}Base {{\n", name).unwrap();
    //        let used_as_count = |name: &str| members.iter().filter_map(|m| m.len.as_ref()).find(|len| len.as_str().split(",").find(|&s| s == name).is_some()).is_some();
    //        for &member in &members {
    //            let mut indirections = member.pointer_info();
    //            let prefix = if all_pub_safe(indirections.iter()) && !used_as_count(member.name.as_str()) {
    //                "pub "
    //            } else {
    //                ""
    //            };
    //            write!(&mut safe_out_file, "    {}{}: ", prefix, &member.name).unwrap();
    //            for indirection in &indirections {
    //                indirection.fmt_begin(&mut safe_out_file).unwrap();
    //            }
    //            safe_out_file.write_all(member.ty.ty.as_bytes()).unwrap();
    //            while let Some(indirection) = indirections.pop_back() {
    //                indirection.fmt_end(&mut safe_out_file).unwrap();
    //            }
    //            safe_out_file.write_all(b",\n").unwrap();
    //        }
    //        safe_out_file.write_all(b"}\n").unwrap();
    //    }
    //}
}
