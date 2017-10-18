extern crate vulkan_sys as vk_sys;

pub mod mem;
pub mod handles;
use mem::VkOwned;
use vk_sys::{ ffi };

pub type VkResult<T> = Result<T, ffi::VkResult>;
