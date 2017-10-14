#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

pub mod flags {
    use ::std::marker::PhantomData;
    use ::std::ops::{ BitAnd, BitOr };

    #[repr(C)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct Flags<T, Repr: BitAnd + BitOr> {
        repr: Repr,
        flag_type: PhantomData<T>,
    }

    impl<T, Repr: BitAnd + BitOr> Flags<T, Repr> {
        #[inline(always)]
        pub fn new(repr: Repr) -> Flags<T, Repr> {
            Flags {
                repr: repr,
                flag_type: PhantomData,
            }
        }
    }

    impl<T, Repr: BitAnd<Output=Repr> + BitOr> BitAnd for Flags<T, Repr> {
        type Output = Self;
        #[inline(always)]
        fn bitand(self, rhs: Self) -> Self {
            Flags::new(self.repr & rhs.repr)
        }
    }

    impl<T, Repr: BitAnd + BitOr<Output=Repr>> BitOr for Flags<T, Repr> {
        type Output = Self;
        #[inline(always)]
        fn bitor(self, rhs: Self) -> Self {
            Flags::new(self.repr | rhs.repr)
        }
    }

    impl<T, Repr: BitAnd + BitOr + ::std::fmt::Binary> ::std::fmt::Debug for Flags<T, Repr> {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
            write!(f, "{:#b}", self.repr)
        }
    }
}

pub mod handle {
    use ::std::marker::PhantomData;
    use super::VK_NULL_HANDLE;

    pub trait NullableHandle {
        fn null() -> Self;
    }

    impl<T> NullableHandle for *mut T {
        #[inline(always)]
        fn null() -> *mut T {
            ::std::ptr::null_mut()
        }
    }

    impl<T> NullableHandle for *const T {
        #[inline(always)]
        fn null() -> *const T {
            ::std::ptr::null()
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub struct NondispatchableHandle<T> {
        handle: u64,
        handle_type: PhantomData<*mut T>,
    }

    impl<T> ::std::fmt::Debug for NondispatchableHandle<T> {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
            write!(f, "0x{:x}", self.handle)
        }
    }

    impl<T> NondispatchableHandle<T> {
        #[inline(always)]
        fn new(handle: u64) -> NondispatchableHandle<T> {
            NondispatchableHandle {
                handle: handle,
                handle_type: PhantomData,
            }
        }
    }

    impl<T> NullableHandle for NondispatchableHandle<T> {
        #[inline(always)]
        fn null() -> NondispatchableHandle<T> {
            NondispatchableHandle::new(VK_NULL_HANDLE as u64)
        }
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn sizeof_nondispatchable_handle() {
            use ::std::mem;
            assert_eq!(mem::size_of::<::ffi::VkImage>(), mem::size_of::<::libc::uint64_t>());
        }
    }
}

pub use self::handle::NullableHandle;

macro_rules! vk_flags {
    ($name: ident, $representation: ty, $flag_type: ty) => {
        pub type $name = flags::Flags<$flag_type, $representation>;

        impl Into<$name> for $flag_type {
            #[inline(always)]
            fn into(self) -> $name {
                flags::Flags::new(self.0 as $representation)
            }
        }
    };
}

macro_rules! vk_non_dispatchable_handle {
    ($name: ident, $t: ty) => {
        pub type $name = handle::NondispatchableHandle<$t>;
    };
}

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[link="vulkan"]
extern "system" {
    pub fn vkGetInstanceProcAddr(instance: VkInstance, pName: *const ::libc::c_char) -> PFN_vkVoidFunction;
}
