use ::ffi;
use ::mem::{ VkSlice, NTV };

use ffi::VkStructureType;
use libc;
use libc::c_char;
use std::ffi::CStr;
use std::{ fmt, ptr };

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VkStructInfo<'a> {
    ty: VkStructureType,
    next: Option<&'a VkStructInfo<'a>>,
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
#[derive(Debug, Clone)]
pub struct VkStructInstance<'a, T> {
    struct_info: VkStructInfo<'a>,
    data: T,
}

impl<'a, T> VkStructInstance<'a, T> {
    pub fn set_next<N: 'a>(&mut self, next: Option<&'a VkStructInstance<'a, N>>) {
        self.struct_info.next = next.map(|n| &n.struct_info);
    }
}

impl<'a, T: VkStruct> From<T> for VkStructInstance<'a, T> {
    #[inline(always)]
    fn from(data: T) -> VkStructInstance<'a, T> {
        VkStructInstance {
            struct_info: T::default_struct_info(),
            data: data,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VkApplicationInfoBase<'a> {
    pub application_name: Option<&'a NTV<libc::c_char>>,
    pub application_version: u32,
    pub engine_name: Option<&'a NTV<libc::c_char>>,
    pub engine_version: u32,
    pub api_version: u32,
}

impl<'a> VkStruct for VkApplicationInfoBase<'a> {
    #[inline]
    fn structure_type() -> VkStructureType {
        VkStructureType::VK_STRUCTURE_TYPE_APPLICATION_INFO
    }
}

pub type VkApplicationInfo<'a> = VkStructInstance<'a, VkApplicationInfoBase<'a>>;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VkInstanceCreateInfoBase<'a> {
    pub flags: ::ffi::VkInstanceCreateFlags,
    pub application_info: Option<&'a VkApplicationInfo<'a>>,
    pub enabled_layer_names: VkSlice<'a, &'a NTV<c_char>, u32>,
    pub enabled_extension_names: VkSlice<'a, &'a NTV<c_char>, u32>,
}

impl<'a> VkStruct for VkInstanceCreateInfoBase<'a> {
    #[inline]
    fn structure_type() -> VkStructureType {
        VkStructureType::VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO
    }
}

pub type VkInstanceCreateInfo<'a> = VkStructInstance<'a, VkInstanceCreateInfoBase<'a>>;

impl<'a> Into<&'a ::ffi::VkInstanceCreateInfo> for &'a VkInstanceCreateInfo<'a> {
    fn into(self) -> &'a ::ffi::VkInstanceCreateInfo {
        unsafe {
            ::std::mem::transmute(self)
        }
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
    fn instance_create_info_size() {
        assert_sizes!(super::VkInstanceCreateInfo, ::ffi::VkInstanceCreateInfo);
    }

    #[test]
    fn application_info_size() {
        assert_sizes!(super::VkApplicationInfo, ::ffi::VkApplicationInfo);
    }
}
