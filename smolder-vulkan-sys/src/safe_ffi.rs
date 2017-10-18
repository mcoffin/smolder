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
    ($base: ident $(($($p:tt)+))*, ($name: ident, $sty: expr)) => {
        pub type $name $( < $( $p )+ > )* = VkStructInstance<'a, $base $(< $( $p )+ >)*>;
        impl $( < $( $p )+ > )* VkStruct for $base $( < $( $p )+ > )* {
            #[inline(always)]
            fn structure_type() -> ::ffi::VkStructureType {
                $sty
            }
        }
    };
    ($base: ty, ($name: ident, $sty: expr)) => {
        pub type $name<'a>  = VkStructInstance<'a, $base>;
        impl VkStruct for $name {
            #[inline(always)]
            fn structure_type() -> ::ffi::VkStructureType {
                $sty
            }
        }
    };
}

use self::VkStructureType::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkApplicationInfoBase<'a> {
    pub application_name: Option<&'a NTV<libc::c_char>>,
    pub application_version: libc::uint32_t,
    pub engine_name: Option<&'a NTV<libc::c_char>>,
    pub engine_version: libc::uint32_t,
    pub api_version: libc::uint32_t,
}
vk_extendable_struct!(VkApplicationInfoBase('a), (VkApplicationInfo, VK_STRUCTURE_TYPE_APPLICATION_INFO));

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkInstanceCreateInfoBase<'a> {
    pub flags: ::ffi::VkInstanceCreateFlags,
    pub application_info: Option<&'a VkApplicationInfo<'a>>,
    pub enabled_layer_names: VkSlice<'a, &'a NTV<c_char>, libc::uint32_t>,
    pub enabled_extension_names: VkSlice<'a, &'a NTV<c_char>, libc::uint32_t>
}
vk_extendable_struct!(VkInstanceCreateInfoBase('a), (VkInstanceCreateInfo, VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO));

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VkBufferViewCreateInfoBase<'a> {
    pub flags: ::ffi::VkBufferViewCreateFlags,
    pub buffer: ::ffi::VkBuffer<'a>,
    pub format: ::ffi::VkFormat,
    pub offset: ::ffi::VkDeviceSize,
    pub range: ::ffi::VkDeviceSize,
}
vk_extendable_struct!(VkBufferViewCreateInfoBase('a), (VkBufferViewCreateInfo, VK_STRUCTURE_TYPE_BUFFER_VIEW_CREATE_INFO));

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
