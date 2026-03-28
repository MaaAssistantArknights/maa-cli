#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

#[macro_use]
mod link;

/// Raw binding of MaaCore API
pub mod binding;

#[cfg(test)]
mod tests {
    use super::binding;

    /// Verify that the null-size sentinel matches MaaCore's definition (`u64::MAX`).
    #[cfg(not(feature = "runtime"))]
    #[test]
    fn null_size_sentinel() {
        assert_eq!(unsafe { binding::AsstGetNullSize() }, u64::MAX);
    }

    /// Verify that `AsstGetVersion` returns a non-null pointer when MaaCore is linked.
    #[cfg(not(feature = "runtime"))]
    #[test]
    fn get_version_non_null() {
        assert!(!unsafe { binding::AsstGetVersion() }.is_null());
    }

    /// Before any `load()` call the library must report as not loaded.
    #[cfg(feature = "runtime")]
    #[test]
    fn initially_not_loaded() {
        assert!(!binding::loaded());
    }

    /// Loading from a nonexistent path must fail gracefully.
    #[cfg(feature = "runtime")]
    #[test]
    fn load_nonexistent_returns_err() {
        assert!(binding::load("/this/library/does_not_exist.so").is_err());
    }
}
