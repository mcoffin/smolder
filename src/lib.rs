#![cfg_attr(feature = "nightly", feature(const_fn))]

extern crate libc;

pub mod bitmask;
pub mod ffi;
pub mod types;

pub use bitmask::*;

/// Macro equivalent of `VK_MAKE_VERSION` preprocessor macro
#[macro_export] macro_rules! vk_make_version {
    ($major: expr, $minor: expr, $patch: expr) => {
        ($major << 22) | ($minor << 12) | $patch
    }
}

#[cfg(feature = "nightly")]
/// Makes a 32-bit unsigned integer representing an api version for vulkan.
/// Effectively a (potentially) more performant version of the
/// `vk_make_version!` macro. since the bitshift operations can be
/// evaluated at compile-time.
pub const fn make_version(major: u8, minor: u8, patch: u8) -> u32 {
    vk_make_version!(major as u32, minor as u32, patch as u32)
}
