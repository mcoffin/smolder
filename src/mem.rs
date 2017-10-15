use vk_sys::safe_ffi::VkAllocationCallbacks;

pub struct VkOwned<'a, T: 'a, UserData: 'a, F: FnMut(&mut T, &VkAllocationCallbacks<UserData>)> {
    handle: &'a mut T,
    allocation_callbacks: &'a VkAllocationCallbacks<'a, UserData>,
    destroy_fn: F,
}

impl<'a, T, UserData, F: FnMut(&mut T, &VkAllocationCallbacks<UserData>)> VkOwned<'a, T, UserData, F> {
    pub unsafe fn new(handle: &'a mut T, allocation_callbacks: &'a VkAllocationCallbacks<'a, UserData>, destroy_fn: F) -> VkOwned<'a, T, UserData, F> {
        VkOwned {
            handle: handle,
            allocation_callbacks: allocation_callbacks,
            destroy_fn: destroy_fn,
        }
    }
}

impl<'a, T, UserData, F: FnMut(&mut T, &VkAllocationCallbacks<UserData>)> Drop for VkOwned<'a, T, UserData, F> {
    fn drop(&mut self) {
        (self.destroy_fn)(self.handle, self.allocation_callbacks);
    }
}
