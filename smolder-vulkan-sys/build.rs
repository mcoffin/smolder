extern crate bindgen;
extern crate vulkan_api_registry;
extern crate xml;

use std::{ env, fs, io, path };
use path::Path;
use vulkan_api_registry::{ EnumsInfo, EnumsType, HandleType, ParseResult, Registry, TypeInfo };
use xml::reader::EventReader;

fn parse_registry<P: AsRef<Path>>(p: P) -> io::Result<ParseResult<Registry>> {
    let always_true = |_: &str| true;
    let events = try!(fs::File::open(p.as_ref()).map(|f| EventReader::new(f).into_iter()));
    Ok(Registry::new(events, &always_true, &always_true))
}

fn main() {
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
    let bindings = {
        let mut bindings = bindgen::Builder::default()
            .header("wrapper.h")
            .ignore_functions() // We don't want to generate functions since we don't want vulkan-loader
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
}
