#![cfg_attr(feature = "const-version", feature(const_fn))]

extern crate libc;

pub mod types;

#[macro_export] macro_rules! vk_make_version {
    ($major: expr, $minor: expr, $patch: expr) => {
        ($major << 22) | ($minor << 12) | $patch
    }
}

#[cfg(feature = "const-version")]
pub const fn make_version(major: u8, minor: u8, patch: u8) -> u32 {
    vk_make_version!(major as u32, minor as u32, patch as u32)
}
