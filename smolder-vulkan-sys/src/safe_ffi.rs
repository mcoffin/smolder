use ::ffi;

use ffi::VkStructureType;
use libc;
use libc::c_char;
use std::ffi::CStr;
use std::{ fmt, ptr };

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VkStructInfo<'a> {
    pub ty: VkStructureType,
    pub next: Option<&'a VkStructInfo<'a>>,
}

pub trait VkStruct {
    fn structure_type() -> VkStructureType;

    #[inline(always)]
    fn default_struct_info<'a>() -> VkStructInfo<'a>{
        VkStructInfo {
            ty: <Self as VkStruct>::structure_type(),
            next: Default::default(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VkApplicationInfo<'a> {
    pub struct_info: VkStructInfo<'a>,
    application_name: *const libc::c_char,
    pub application_version: u32,
    engine_name: *const libc::c_char,
    pub engine_version: u32,
    pub api_version: u32,
}

fn unwrap_cstr(cs: Option<&CStr>) -> *const libc::c_char {
    cs.map(|s| s.as_ptr()).unwrap_or(ptr::null())
}

impl<'a> VkApplicationInfo<'a> {
    pub fn new(application_name: Option<&'a CStr>, application_version: u32, engine_name: Option<&'a CStr>, engine_version: u32, api_version: u32) -> VkApplicationInfo<'a> {
        VkApplicationInfo {
            struct_info: <Self as VkStruct>::default_struct_info(),
            application_name: unwrap_cstr(application_name),
            application_version: application_version,
            engine_name: unwrap_cstr(engine_name),
            engine_version: engine_version,
            api_version: api_version,
        }
    }
}

impl<'a> VkStruct for VkApplicationInfo<'a> {
    #[inline]
    fn structure_type() -> VkStructureType {
        VkStructureType::VK_STRUCTURE_TYPE_APPLICATION_INFO
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VkInstanceCreateInfo<'a> {
    pub struct_info: VkStructInfo<'a>,
    pub flags: ::ffi::VkInstanceCreateFlags,
    pub application_info: Option<&'a VkApplicationInfo<'a>>,
    enabled_layer_count: u32,
    enabled_layer_names: *const *const c_char,
    enabled_extension_count: u32,
    enabled_extension_names: *const *const c_char,
}

impl<'a> VkInstanceCreateInfo<'a> {
    pub unsafe fn new(flags: ::ffi::VkInstanceCreateFlags, application_info: Option<&'a VkApplicationInfo<'a>>, enabled_layers: &'a [*const c_char], enabled_extensions: &'a [*const c_char]) -> VkInstanceCreateInfo<'a> {
        VkInstanceCreateInfo {
            struct_info: <Self as VkStruct>::default_struct_info(),
            flags: flags,
            application_info: application_info,
            enabled_layer_count: enabled_layers.len() as u32,
            enabled_layer_names: enabled_layers.as_ptr(),
            enabled_extension_count: enabled_extensions.len() as u32,
            enabled_extension_names: enabled_extensions.as_ptr(),
        }
    }
}

impl<'a> Into<&'a ::ffi::VkInstanceCreateInfo> for &'a VkInstanceCreateInfo<'a> {
    fn into(self) -> &'a ::ffi::VkInstanceCreateInfo {
        unsafe {
            ::std::mem::transmute(self)
        }
    }
}

impl<'a> VkStruct for VkInstanceCreateInfo<'a> {
    #[inline]
    fn structure_type() -> VkStructureType {
        VkStructureType::VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO
    }
}

#[cfg(test)]
mod tests {
    macro_rules! assert_sizes {
        ($a: ty, $b: ty) => {
            assert_eq!(::std::mem::size_of::<$a>(), ::std::mem::size_of::<$b>());
        };
    }

    #[test]
    fn ref_opt_ref_ptr_size() {
        use libc::{ c_char, c_void };
        use std::ffi::CStr;
        assert_sizes!(&c_char, *const c_char);
        assert_sizes!(Option<&c_char>, *const c_char);
        assert_sizes!(Option<&super::VkStructInfo>, *const c_void);
    }

    #[test]
    fn application_info_size() {
        assert_sizes!(super::VkApplicationInfo, ::ffi::VkApplicationInfo);
    }

    #[test]
    fn instance_create_info_size() {
        assert_sizes!(super::VkInstanceCreateInfo, ::ffi::VkInstanceCreateInfo);
    }
}
