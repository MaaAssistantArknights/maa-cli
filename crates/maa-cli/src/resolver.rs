use std::sync::atomic;

use maa_question::prelude::*;

static BATCH_MODE: atomic::AtomicBool = atomic::AtomicBool::new(cfg!(test));

/// Initialize the global resolver mode.
///
/// Must be called once before any code that uses [`with_global_resolver`] or [`ask`].
pub fn init(batch: bool) {
    BATCH_MODE.store(batch, std::sync::atomic::Ordering::SeqCst);
}

pub fn is_batch() -> bool {
    BATCH_MODE.load(std::sync::atomic::Ordering::SeqCst)
}

pub enum CliResolver {
    Batch(BatchResolver),
    StdIo(IoResolver<StdIo>),
}

impl From<BatchResolver> for CliResolver {
    fn from(resolver: BatchResolver) -> Self {
        Self::Batch(resolver)
    }
}

impl From<IoResolver<StdIo>> for CliResolver {
    fn from(resolver: IoResolver<StdIo>) -> Self {
        Self::StdIo(resolver)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CliResolverError {
    #[error(transparent)]
    Batch(#[from] BatchError),
    #[error(transparent)]
    StdIo(#[from] std::io::Error),
}
macro_rules! forward_resolve_impl {
    ($question:ty) => {
        impl Resolve<$question> for CliResolver {
            type Error = CliResolverError;

            fn resolve(
                &mut self,
                question: $question,
            ) -> Result<<$question as Question>::Answer, Self::Error> {
                match self {
                    CliResolver::Batch(resolver) => {
                        resolver.resolve(question).map_err(CliResolverError::Batch)
                    }
                    CliResolver::StdIo(resolver) => {
                        resolver.resolve(question).map_err(CliResolverError::StdIo)
                    }
                }
            }
        }
    };
    ($($question:ty),* $(,)?) => {
        $(forward_resolve_impl!($question);)*
    };
}

forward_resolve_impl!(
    Confirm,
    Inquiry<i32>,
    Inquiry<f32>,
    Inquiry<String>,
    SelectD<i32>,
    SelectD<f32>,
    SelectD<String>,
);

/// Access the resolver.
///
/// Creates a short-lived resolver on each call so that `StdinLock` / `StdoutLock`
/// are not held between calls.
///
/// The resolver mode defaults to batch in tests and stdio otherwise.
/// Call [`init`] from the CLI entry point to override that default for the
/// current process.
pub fn with_global_resolver<F, R>(f: F) -> R
where
    F: FnOnce(&mut CliResolver) -> R,
{
    let batch = BATCH_MODE.load(std::sync::atomic::Ordering::SeqCst);

    if batch {
        f(&mut CliResolver::Batch(BatchResolver::default()))
    } else {
        f(&mut CliResolver::StdIo(IoResolver(StdIo::new())))
    }
}

pub fn ask<T>(question: T) -> Result<<T as Question>::Answer, CliResolverError>
where
    T: Question,
    CliResolver: Resolve<T, Error = CliResolverError>,
{
    with_global_resolver(|resolver| resolver.resolve(question))
}
