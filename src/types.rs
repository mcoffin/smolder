use std::{ fmt, ops, ptr };
use libc::*;

macro_rules! smolder_ffi_handle {
    ($name: ident) => {
        #[repr(C)]
        pub struct $name (*mut ());

        impl $name {
            #[inline(always)]
            pub unsafe fn null() -> $name {
                $name(ptr::null_mut())
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                write!(f, "0x{:x}", self.0 as usize)
            }
        }
    };
}

const VK_NULL_HANDLE: u64 = 0x0;

macro_rules! smolder_ffi_handle_nondispatchable {
    ($name: ident) => {
        #[repr(C)]
        pub struct $name (u64);

        impl $name {
            #[inline(always)]
            pub unsafe fn null() -> $name {
                $name(VK_NULL_HANDLE)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                write!(f, "0x{:x}", self.0)
            }
        }
    };
}

pub trait Bitmask<RHS = Self> {
    fn intersects(self, rhs: RHS) -> bool;
    fn subset(self, rhs: RHS) -> bool;
}

impl<T: ops::BitAnd<Output=T> + Eq + Default + Copy> Bitmask for T {
    fn intersects(self, other: Self) -> bool {
        self.bitand(other).ne(&Default::default())
    }

    fn subset(self, other: Self) -> bool {
        self.bitand(other).eq(&self)
    }
}

macro_rules! smolder_ffi_bitmask {
    ($mname: ident, $representation: ty, $( $name: ident, $value: expr ),*) => {
        #[repr(C)]
        #[derive(Clone, Copy)]
        pub struct $mname ($representation);

        impl Into<$representation> for $mname {
            fn into(self) -> $representation {
                self.0
            }
        }

        impl PartialEq for $mname {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }

        impl Eq for $mname {}

        impl ops::BitOr for $mname {
            type Output = Self;
            fn bitor(self, rhs: Self) -> Self {
                $mname(self.0 | rhs.0)
            }
        }

        impl ops::BitAnd for $mname {
            type Output = Self;
            fn bitand(self, rhs: Self) -> Self {
                $mname(self.0 & rhs.0)
            }
        }

        impl Default for $mname {
            fn default() -> Self {
                $mname(0)
            }
        }

        $(
            pub const $name: $mname = $mname($value);
        )*
    };
}

// basetype

pub type VkSampleMask = uint32_t;
pub type VkBool32 = uint32_t;
pub type VkFlags = uint32_t;
pub type VkDeviceSize = uint32_t;

// handle

smolder_ffi_handle!(VkInstance);
smolder_ffi_handle!(VkPhysicalDevice);
smolder_ffi_handle!(VkDevice);
smolder_ffi_handle!(VkQueue);
smolder_ffi_handle_nondispatchable!(VkSemaphore);
smolder_ffi_handle!(VkCommandBuffer);
smolder_ffi_handle_nondispatchable!(VkFence);
smolder_ffi_handle_nondispatchable!(VkDeviceMemory);
smolder_ffi_handle_nondispatchable!(VkBuffer);
smolder_ffi_handle_nondispatchable!(VkImage);
smolder_ffi_handle_nondispatchable!(VkEvent);
smolder_ffi_handle_nondispatchable!(VkQueryPool);
smolder_ffi_handle_nondispatchable!(VkBufferView);
smolder_ffi_handle_nondispatchable!(VkImageView);
smolder_ffi_handle_nondispatchable!(VkShaderModule);
smolder_ffi_handle_nondispatchable!(VkPipelineCache);
smolder_ffi_handle_nondispatchable!(VkPipelineLayout);
smolder_ffi_handle_nondispatchable!(VkRenderPass);
smolder_ffi_handle_nondispatchable!(VkPipeline);
smolder_ffi_handle_nondispatchable!(VkDescriptorSetLayout);
smolder_ffi_handle_nondispatchable!(VkSampler);
smolder_ffi_handle_nondispatchable!(VkDescriptorPool);
smolder_ffi_handle_nondispatchable!(VkDescriptorSet);
smolder_ffi_handle_nondispatchable!(VkFramebuffer);
smolder_ffi_handle_nondispatchable!(VkCommandPool);

// enum

#[repr(C)]
#[derive(Clone, Copy)]
pub enum VkFrontFace {
    ZERO = 0,
    ONE = 1,
}

// bitmask

smolder_ffi_bitmask! {
    VkCullModeFlags,
    VkFlags,
    CULL_MODE_NONE, 0,
    CULL_MODE_FRONT, 0b1,
    CULL_MODE_BACK, 0b10
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_bitmask_implemented_by_bitmasks() {
        let both = CULL_MODE_FRONT | CULL_MODE_BACK;
        assert!(CULL_MODE_FRONT.intersects(CULL_MODE_FRONT));
        assert!(!CULL_MODE_FRONT.intersects(CULL_MODE_BACK));
        assert!(CULL_MODE_FRONT.intersects(both) && CULL_MODE_FRONT.subset(both));
        assert!(CULL_MODE_BACK.intersects(both) && CULL_MODE_FRONT.subset(both));
        assert!(CULL_MODE_FRONT != both);
        assert!(CULL_MODE_BACK != both);
    }
}
