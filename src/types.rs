use std::fmt;
use std::marker::PhantomData;

macro_rules! smolder_handle {
    ($name: ident) => {
        pub enum $name {}
    };
    ($name: ident, $parent: ty) => {
        // TODO
    };
}

macro_rules! smolder_handle_nondispatchable {
    ($name: ident) => {
        #[repr(C)]
        #[derive(Eq, PartialEq)]
        pub struct $name (u64);

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                write!(f, "0x{:x}", self.0)
            }
        }
    };
    ($name: ident, $parent: ty) => {
        #[repr(C)]
        #[derive(Hash)]
        pub struct $name<'parent> {
            raw_handle: u64,
            parent: PhantomData<&'parent $parent>,
        }

        impl<'parent> fmt::Debug for $name<'parent> {
            fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                write!(f, "0x{:x}", self.raw_handle)
            }
        }
    };
}

smolder_handle!(Device);
smolder_handle_nondispatchable!(Semaphore, Device);
