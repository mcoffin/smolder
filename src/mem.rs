pub struct VkOwned<T, F: FnOnce(&mut T)> {
    handle: *mut T,
    destroy_fn: F,
}

impl<T, F: FnMut(&mut T)> VkOwned<T, F> {
    pub unsafe fn new(handle: *mut T, destroy_fn: F) -> VkOwned<T, F> {
        VkOwned {
            handle: handle,
            destroy_fn: destroy_fn,
        }
    }
}

impl<T, F: FnMut(&mut T)> Drop for VkOwned<T, F> {
    fn drop(&mut self) {
        (self.destroy_fn)(unsafe { &mut *self.handle });
    }
}
