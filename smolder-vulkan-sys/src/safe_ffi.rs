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
#[derive(Debug, Clone, Copy)]
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

macro_rules! vk_extendable_struct {
    (pub struct ($name: ident, $basename: ident) -> ($stype: expr) { $( $mname: ident : $mtype: ty ),* }) => {
        #[repr(C)]
        #[derive(Debug, Clone, Copy)]
        pub struct $basename<'a> {
            $(
                pub $mname: $mtype,
            )*
        }

        impl<'a> VkStruct for $basename<'a> {
            #[inline]
            fn structure_type() -> VkStructureType {
                $stype
            }
        }

        pub type $name<'a> = VkStructInstance<'a, $basename<'a>>;

        impl<'a> Into<&'a ::ffi::$name> for &'a $name<'a> {
            #[inline]
            fn into(self) -> &'a ::ffi::$name {
                use std::mem::transmute;
                unsafe {
                    transmute(self)
                }
            }
        }
    };
}

use self::VkStructureType::*;
vk_extendable_struct! {
    pub struct (VkApplicationInfo, VkApplicationInfoBase) -> (VK_STRUCTURE_TYPE_APPLICATION_INFO) {
        application_name: Option<&'a NTV<libc::c_char>>,
        application_version: libc::uint32_t,
        engine_name: Option<&'a NTV<libc::c_char>>,
        engine_version: libc::uint32_t,
        api_version: libc::uint32_t
    }
}
vk_extendable_struct! {
    pub struct (VkInstanceCreateInfo, VkInstanceCreateInfoBase) -> (VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO) {
        flags: ::ffi::VkInstanceCreateFlags,
        application_info: Option<&'a VkApplicationInfo<'a>>,
        enabled_layer_names: VkSlice<'a, &'a NTV<c_char>, libc::uint32_t>,
        enabled_extension_names: VkSlice<'a, &'a NTV<c_char>, libc::uint32_t>
    }
}

#[repr(C)]
pub struct VkAllocationCallbacks<'a, UserData: 'a + ?Sized> {
    pub user_data: &'a mut UserData,
    pub allocation: ::ffi::PFN_vkAllocationFunction,
    pub reallocation: ::ffi::PFN_vkReallocationFunction,
    pub free: ::ffi::PFN_vkFreeFunction,
    pub internal_allocation: ::ffi::PFN_vkInternalAllocationNotification,
    pub internal_free: ::ffi::PFN_vkInternalFreeNotification,
}

#[cfg(test)]
mod tests {
    macro_rules! assert_sizes {
        ($a: ty, $b: ty) => {
            assert_eq!(::std::mem::size_of::<$a>(), ::std::mem::size_of::<$b>());
        };
    }

    #[test]
    fn application_info_size() {
        assert_sizes!(super::VkApplicationInfo, ::ffi::VkApplicationInfo);
    }

    #[test]
    fn instance_create_info_size() {
        assert_sizes!(super::VkInstanceCreateInfo, ::ffi::VkInstanceCreateInfo);
    }

    #[test]
    fn allocation_callbacks_size() {
        assert_sizes!(super::VkAllocationCallbacks<()>, ::ffi::VkAllocationCallbacks);
    }
}
