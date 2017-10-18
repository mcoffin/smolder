pub struct VkOwned<T: Copy, F: FnOnce(T)> {
    handle: T,
    destroy_fn: Option<F>,
}

impl<T: Copy, F: FnOnce(T)> VkOwned<T, F> {
    pub unsafe fn new(handle: T, destroy_fn: F) -> VkOwned<T, F> {
        VkOwned {
            handle: handle,
            destroy_fn: destroy_fn,
        }
    }
}

impl<T: Copy, F: FnOnce(T)> Drop for VkOwned<T, F> {
    fn drop(&mut self) {
        let destroy_fn = self.destroy_fn.take().unwrap();
        destroy_fn(self.handle);
    }
}
