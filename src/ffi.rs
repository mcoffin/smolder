use std::{ fmt, ops, ptr };
use libc::*;

/// `null` value for the internals of *non-dispatchable* vulkan handles
const VK_NULL_HANDLE: u64 = 0x0;

/// All types which can be unsafely temporarily initialized with a null value
/// should implement this trait
pub trait UnsafelyNullableHandle {
    unsafe fn null() -> Self;
}

/// Defines a *dispatchable* vulkan handle (os-sized pointer value). The
/// newly-defined handle will implement the Debug trait, and contain
/// an impl
macro_rules! smolder_ffi_handle {
    ($name: ident) => {
        #[repr(C)]
        pub struct $name (*mut ());

        impl UnsafelyNullableHandle for $name {
            #[inline(always)]
            unsafe fn null() -> $name {
                $name(ptr::null_mut())
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                write!(f, "0x{:x}", self.0 as usize)
            }
        }
    };
}

macro_rules! smolder_ffi_handle_nondispatchable {
    ($name: ident) => {
        #[repr(C)]
        pub struct $name (u64);

        impl UnsafelyNullableHandle for $name {
            #[inline(always)]
            unsafe fn null() -> $name {
                $name(VK_NULL_HANDLE)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                write!(f, "0x{:x}", self.0)
            }
        }
    };
}

macro_rules! smolder_ffi_bitmask {
    ($mname: ident, $representation: ty, $( $name: ident, $value: expr, )*) => {
        #[repr(C)]
        #[derive(Debug, Clone, Copy)]
        pub struct $mname ($representation);

        impl Into<$representation> for $mname {
            fn into(self) -> $representation {
                self.0
            }
        }

        impl PartialEq for $mname {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }

        impl Eq for $mname {}

        impl ops::BitOr for $mname {
            type Output = Self;
            fn bitor(self, rhs: Self) -> Self {
                $mname(self.0 | rhs.0)
            }
        }

        impl ops::BitAnd for $mname {
            type Output = Self;
            fn bitand(self, rhs: Self) -> Self {
                $mname(self.0 & rhs.0)
            }
        }

        impl Default for $mname {
            fn default() -> Self {
                $mname(0)
            }
        }

        $(
            pub const $name: $mname = $mname($value);
        )*
    };
}

include!(concat!(env!("OUT_DIR"), "/vk.rs"));
