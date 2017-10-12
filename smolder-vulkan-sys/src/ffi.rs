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
        fn bitand(mut self, rhs: Self) -> Self {
            self.repr = self.repr & rhs.repr;
            self
        }
    }

    impl<T, Repr: BitAnd + BitOr<Output=Repr>> BitOr for Flags<T, Repr> {
        type Output = Self;
        fn bitor(mut self, rhs: Self) -> Self {
            self.repr = self.repr | rhs.repr;
            self
        }
    }

    impl<T> ::std::fmt::Debug for Flags<T, super::VkFlags> {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
            write!(f, "{:#b}", self.repr)
        }
    }
}

macro_rules! vk_flags {
    ($name: ident, $representation: ty, $flag_type: ty) => {
        pub type $name = flags::Flags<$flag_type, $representation>;

        impl Into<$name> for $flag_type {
            fn into(self) -> $name {
                flags::Flags::new(self.0 as $representation)
            }
        }
    };
}
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
