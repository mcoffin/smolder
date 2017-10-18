use std::mem::transmute;
use vk_sys::ffi;
use vk_sys::ffi::handle::NondispatchableHandle;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NondispatchableHandleRef<'a, T> {
    handle: u64,
    phantom_ref: PhantomData<&'a mut NondispatchableHandle<T>>,
}

impl<'a, T> NondispatchableHandleRef<'a, T> {
    #[inline(always)]
    unsafe fn unsafe_handle(self) -> NondispatchableHandle<T> {
        transmute(self)
    }
}

type VkInstance<'a> = &'a mut ffi::VkInstance_T;
type VkCommandPool<'a> = NondispatchableHandleRef<'a, ffi::VkCommandPool_T>;

#[cfg(test)]
mod tests {
    use std::mem::size_of;
    #[test]
    fn non_dispatchable_handle_ref_size() {
        let handle_size = size_of::<NondispatchableHandle<ffi::VkCommandPool_T>>();
        let ref_size = size_of::<NondispatchableHandleRef<ffi::VkCommandPool_T>>();
        assert_eq!(handle_size, ref_size);
    }
}
