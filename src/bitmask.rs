use std::ops;

/// Extra utility functions implemented by types that behave
/// like bitmasks
pub trait Bitmask<RHS = Self> {
    /// Returns true if any of the same bits are "on" for both bitmasks
    fn intersects(self, rhs: RHS) -> bool;
    /// Returns true if this is a subset of `rhs`.
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_bitmask_implemented_by_bitmasks() {
        use ::types::*;
        let both = CULL_MODE_FRONT | CULL_MODE_BACK;
        assert!(CULL_MODE_FRONT.intersects(CULL_MODE_FRONT));
        assert!(!CULL_MODE_FRONT.intersects(CULL_MODE_BACK));
        assert!(CULL_MODE_FRONT.intersects(both) && CULL_MODE_FRONT.subset(both));
        assert!(CULL_MODE_BACK.intersects(both) && CULL_MODE_FRONT.subset(both));
        assert!(CULL_MODE_FRONT != both);
        assert!(CULL_MODE_BACK != both);
    }
}
