use libc;
use std::fmt::Debug;
use std::ffi::CStr;
use std::mem::{ size_of, transmute };
use std::marker::PhantomData;

pub struct NTVIter<'a, T: 'a + Sized + Default + PartialEq> {
    arr: &'a NTV<T>,
}

impl<'a, T: Sized + Default + PartialEq> Iterator for NTVIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        let head = self.arr.head_data();
        if head == &Default::default() {
            None
        } else {
            unsafe {
                self.arr = NTV::new_unchecked((self.arr.as_ptr() as usize + size_of::<T>()) as *const T);
            }
            Some(head)
        }
    }
}

// TODO: This should really be a dynamically sized type. Instead, we just don't implement "copy",
// and make sure to only hand out references from this module
/// Zero-sized, but really dynamically sized, type for representing null-terminated `Vec`s.
#[repr(C)]
pub struct NTV<T: Sized + Default> (PhantomData<T>);

impl<T: Sized + Default + Debug + PartialEq> Debug for NTV<T> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        f.debug_list().entries(self.into_iter()).finish()
    }
}

impl<T: Sized + Default> NTV<T> {
    /// Unsafely creates a new NTV by assuing that the given pointer points to some memory that
    /// eventually will have a null value.
    #[inline]
    pub unsafe fn new_unchecked<'a>(head: *const T) -> &'a NTV<T> {
        transmute(head)
    }

    /// Gets a pointer to the head data
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.head_data() as *const T
    }

    /// Gets a reference to the head data
    #[inline]
    fn head_data(&self) -> &T {
        unsafe {
            transmute(self)
        }
    }
}

/// `CStr` can be safely turned in to an NTV since it should have already been checked for length
impl<'a> From<&'a CStr> for &'a NTV<libc::c_char> {
    fn from(s: &'a CStr) -> &'a NTV<libc::c_char> {
        unsafe {
            NTV::new_unchecked(s.as_ptr())
        }
    }
}

impl<'a, T: 'a + Sized + Default + PartialEq> IntoIterator for &'a NTV<T> {
    type Item = &'a T;
    type IntoIter = NTVIter<'a, T>;
    #[inline(always)]
    fn into_iter(self) -> NTVIter<'a, T> {
        NTVIter {
            arr: self
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VkSlice<'a, T: 'a, S: Copy> {
    len: S,
    ptr: Option<&'a T>,
}

impl<'a, T: 'a> From<&'a [T]> for VkSlice<'a, T, u32> {
    fn from(slice: &'a [T]) -> VkSlice<'a, T, u32> {
        let len = slice.len();
        if len > (u32::max_value() as usize) {
            panic!("slice length {} is greater than u32::max_value ({})", len, u32::max_value());
        }
        VkSlice {
            len: len as u32,
            ptr: slice.first(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libc;
    use std;

    #[test]
    fn ntv_size() {
        use libc::c_char;
        use std::mem::size_of;
        let ptr_size = size_of::<*mut c_char>();
        let ntv_size = size_of::<&NTV<c_char>>();
        assert_eq!(ptr_size, ntv_size);
    }

    #[test]
    fn optional_ntv_size() {
        use libc::c_char;
        use std::mem::size_of;
        let ptr_size = size_of::<*mut c_char>();
        let ntv_size = size_of::<Option<&NTV<c_char>>>();
        assert_eq!(ptr_size, ntv_size);
    }
}
