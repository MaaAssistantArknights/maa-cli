use maa_ffi_types::{AsstBool, AsstId, AsstSize};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("MaaCore returned an error, check its log ({0}) for details")]
    MAAError(std::path::PathBuf),
    #[error("Failed to create Assistant")]
    NullHandle,
    #[error("Buffer too small")]
    BufferTooSmall,
    #[error("The content returned by MaaCore is too large (length > {0})")]
    ContentTooLarge(usize),
    #[error("Input argument contains invalid bytes")]
    InvalidArgument(#[from] maa_ffi_string::Error),
    #[error("Returned value contains invalid bytes")]
    InvalidReturnValue(#[from] std::string::FromUtf8Error),
    #[cfg(all(feature = "runtime", target_os = "windows"))]
    #[error("OS error")]
    OS(#[from] windows_result::Error),
    #[cfg(feature = "runtime")]
    #[error("Failed to load the shared library")]
    LoadError(#[from] libloading::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Specific error produced when an `AsstBool` or `AsstId` FFI call returns its
/// failure sentinel. Carries the path to MaaCore's log file so callers can
/// direct users to it for diagnosis.
///
/// Converts to [`Error::MAAError`] via `From`.
#[derive(Debug)]
pub(crate) struct MaaCoreError(pub(crate) std::path::PathBuf);

impl From<MaaCoreError> for Error {
    fn from(e: MaaCoreError) -> Self {
        Error::MAAError(e.0)
    }
}

/// Specific error produced when an `AsstSize` FFI call returns [`NULL_SIZE`],
/// indicating the caller-provided buffer was too small.
///
/// Converts to [`Error::BufferTooSmall`] via `From`.
#[derive(Debug)]
pub(crate) struct BufferTooSmall;

impl From<BufferTooSmall> for Error {
    fn from(_: BufferTooSmall) -> Self {
        Error::BufferTooSmall
    }
}

/// Converts a raw FFI return value into a `Result` with a precise error type.
///
/// Each `impl` maps a specific sentinel value to the one error that can
/// actually occur for that type, making call sites self-documenting:
///
/// ```text
/// AsstBool  → Ok(()) | Err(MaaCoreError)    (sentinel: 0)
/// AsstSize  → Ok(n)  | Err(FfiBufferTooSmall) (sentinel: u64::MAX)
/// AsstId    → Ok(id) | Err(MaaCoreError)    (sentinel: 0)
/// ```
///
/// # Usage
///
/// - **`.to_maa_result()`** — promotes the specific error to [`Error`] via `From`, returning the
///   crate's `Result<T>`. Use this with `?` at most call sites.
/// - **`.to_result()`** — returns `Result<T, SpecificErr>` for the rare cases where you need to
///   match on the concrete error type (e.g. to remap it to a different `Error` variant).
pub(crate) trait AsstResult: Sized {
    type Return;
    type Err: Into<Error>;

    /// Converts `self` into a `Result` with the precise FFI error type.
    fn to_result(self) -> std::result::Result<Self::Return, Self::Err>;

    /// Converts `self` into the crate's `Result<T>`, promoting the specific
    /// error to [`Error`] via `From`. Equivalent to `.to_result().map_err(Into::into)`.
    fn to_maa_result(self) -> Result<Self::Return> {
        self.to_result().map_err(Into::into)
    }
}

/// Sentinel value returned by MaaCore to indicate a size-related failure.
///
/// Defined as `u64::MAX` in MaaCore.
const NULL_SIZE: AsstSize = AsstSize::MAX;

/// Sentinel value returned by MaaCore to indicate an ID-related failure.
///
/// Defined as `0` in MaaCore.
const INVALID_ID: AsstId = 0;

impl AsstResult for AsstBool {
    type Err = MaaCoreError;
    type Return = ();

    fn to_result(self) -> std::result::Result<(), MaaCoreError> {
        if self == 1 {
            Ok(())
        } else {
            Err(MaaCoreError(crate::get_log_path()))
        }
    }
}

impl AsstResult for AsstSize {
    type Err = BufferTooSmall;
    type Return = Self;

    fn to_result(self) -> std::result::Result<AsstSize, BufferTooSmall> {
        if self == NULL_SIZE {
            Err(BufferTooSmall)
        } else {
            Ok(self)
        }
    }
}

impl AsstResult for AsstId {
    type Err = MaaCoreError;
    type Return = Self;

    fn to_result(self) -> std::result::Result<AsstId, MaaCoreError> {
        if self == INVALID_ID {
            Err(MaaCoreError(crate::get_log_path()))
        } else {
            Ok(self)
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn asst_bool() {
        assert!(matches!(0u8.to_result(), Err(MaaCoreError(_))));
        assert!(matches!(1u8.to_result(), Ok(())));
    }

    #[test]
    fn asst_size() {
        assert!(matches!(NULL_SIZE.to_result(), Err(BufferTooSmall)));
        assert!(matches!(1u64.to_result(), Ok(1u64)));
        #[cfg(not(feature = "runtime"))]
        assert_eq!(unsafe { maa_sys::binding::AsstGetNullSize() }, NULL_SIZE);
    }

    #[test]
    fn asst_id() {
        assert!(matches!(INVALID_ID.to_result(), Err(MaaCoreError(_))));
        assert_eq!(1i32.to_result().unwrap(), 1i32);
    }
}
