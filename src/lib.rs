extern crate vulkan_sys as vk_sys;

pub mod mem;
use mem::VkOwned;
use std::borrow::Borrow;
use std::ffi::CString;
use vk_sys::{ ffi, safe_ffi };

pub enum LoadError {
    FunctionNotLoaded(CString),
}

pub type VkResult<T> = Result<T, ffi::VkResult>;

pub struct Entry {
    create_instance_fn: ffi::PFN_vkCreateInstance,
}

impl Entry {
    pub fn load() -> Result<Entry, LoadError> {
        let fn_name: CString = CString::new("vkCreateInstance").unwrap();
        let create_instance = vk_sys::get_entry_proc_addr(fn_name.borrow())
            .map(|f| Ok(f))
            .unwrap_or_else(|| Err(LoadError::FunctionNotLoaded(fn_name.clone())));
        Ok(Entry {
            create_instance_fn: Some(try!(create_instance)),
        })
    }

    pub fn create_instance<'a T>(
        &self,
        create_info: &safe_ffi::VkInstanceCreateInfo,
        allocation_callbacks: &'a safe_ffi::VkAllocationCallbacks<T>
        ) -> VkResult<VkOwned<ffi::VkInstance_T, impl FnOnce(&mut T)>> {
        use vk_sys::ffi::NullableHandle;
        let create_instance = self.create_instance_fn.unwrap();
        let mut handle: ffi::VkInstance = VkInstance::null();
        let result = unsafe {
            create_instance(create_info, allocation_callbacks, &mut handle)
        };
        let handle = match result {
            ffi::VkResult::VK_SUCCESS => unsafe {
                Ok(handle)
            },
            e => Err(e),
        };
        let handle = handle.and_then(|handle| {
            let destroy_instance = unsafe {
                ffi::vkGetInstanceProcAddr(
            };
        })
    }
}
