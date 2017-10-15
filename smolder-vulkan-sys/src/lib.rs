#![feature(optin_builtin_traits)]
extern crate libc;

pub mod ffi;
pub mod safe_ffi;
pub mod mem;

use std::ffi::CStr;

#[macro_export] macro_rules! vk_make_version {
    ($major: expr, $minor: expr, $patch: expr) => {
        ($major << 22) | ($minor << 12) | $patch
    };
}

#[inline]
pub fn get_entry_proc_addr(name: &CStr) -> ffi::PFN_vkVoidFunction {
    use ffi::*;
    unsafe {
        vkGetInstanceProcAddr(NullableHandle::null(), name.as_ptr())
    }
}
