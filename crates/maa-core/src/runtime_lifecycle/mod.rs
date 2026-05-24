#[cfg(not(feature = "runtime"))]
mod linked;
#[cfg(feature = "runtime")]
mod runtime;

#[cfg(not(feature = "runtime"))]
pub(crate) use linked::*;
#[cfg(feature = "runtime")]
pub(crate) use runtime::*;
