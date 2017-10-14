#[macro_use] extern crate vulkan_sys as vk_sys;

use vk_sys::safe_ffi::VkApplicationInfo;

fn main() {
    use std::{ mem, ptr };
    use std::ffi::CString;
    use std::borrow::Borrow;
    let application_name: CString = CString::new(env!("CARGO_PKG_NAME")).unwrap();
    let application_info = vk_sys::safe_ffi::VkApplicationInfo::new(
        Some(application_name.borrow()),
        vk_make_version!(1, 0, 0),
        None,
        Default::default(),
        vk_make_version!(1, 0, 36));
    let instance_create_info = unsafe {
        vk_sys::safe_ffi::VkInstanceCreateInfo::new(
            Default::default(),
            None,
            &[],
            &[])
    };
    let create_instance = unsafe {
        let fn_name = CString::new("vkCreateInstance").unwrap();
        let create_instance: vk_sys::ffi::PFN_vkCreateInstance = vk_sys::get_entry_proc_addr(fn_name.borrow()).map(|f| mem::transmute(f));
        create_instance.expect("Couldn't load vkCreateInstance")
    };
    let instance = unsafe {
        use vk_sys::ffi::NullableHandle;

        let mut instance: vk_sys::ffi::VkInstance = NullableHandle::null();
        let instance_create_info: &vk_sys::ffi::VkInstanceCreateInfo = (&instance_create_info).into();
        let result = create_instance(
            instance_create_info as *const vk_sys::ffi::VkInstanceCreateInfo,
            ptr::null(), 
            &mut instance as *mut vk_sys::ffi::VkInstance);
        match result {
            vk_sys::ffi::VkResult::VK_SUCCESS => Ok(instance),
            result => Err(result),
        }
    };
    let instance = instance.unwrap();
    let destroy_instance = unsafe {
        let fn_name = CString::new("vkDestroyInstance").unwrap();
        let destroy_instance: vk_sys::ffi::PFN_vkDestroyInstance = vk_sys::ffi::vkGetInstanceProcAddr(instance, fn_name.as_ptr()).map(|f| mem::transmute(f));
        destroy_instance.expect("Couldn't load vkDestroyInstance")
    };
    println!("{:?}", &instance);
    unsafe {
        destroy_instance(instance, ptr::null());
    }
}
