extern crate bindgen;
extern crate vulkan_api_registry;
extern crate xml;

use std::{ env, fs, fmt, io, path };
use std::collections::LinkedList;
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
        bindings = bindings
            .hide_type("VkBufferViewCreateInfo")
            .raw_line(format!("use ::safe_ffi::VkBufferViewCreateInfo;"));
        bindings.generate().unwrap()
    };
    let out_path = env::var("OUT_DIR").map(|ref s| {
        let s: &Path = s.as_ref();
        s.join("bindings.rs")
    }).expect("OUT_DIR should be set");
    bindings.write_to_file(out_path).unwrap();
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
