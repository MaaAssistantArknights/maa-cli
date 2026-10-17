use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, Waker},
};

use crate::{ExecutionReport, Result, events::ExecutionSubscription};

/// A cloneable cooperative cancellation token.
#[derive(Clone, Default)]
pub struct StopToken {
    requested: Arc<AtomicBool>,
}

impl StopToken {
    /// Requests that the current execution stop.
    pub fn request_stop(&self) {
        self.requested.store(true, Ordering::Relaxed);
    }

    /// Returns whether a stop has been requested.
    pub fn is_requested(&self) -> bool {
        self.requested.load(Ordering::Relaxed)
    }
}

/// Owns an execution's ordered event stream and eventual report.
pub struct ExecutionHandle {
    completion: Arc<CompletionState>,
    events: ExecutionSubscription,
}

impl ExecutionHandle {
    pub(crate) fn new(completion: Arc<CompletionState>, events: ExecutionSubscription) -> Self {
        Self { completion, events }
    }

    /// Returns the event stream created before the execution thread started.
    pub fn events(&self) -> &ExecutionSubscription {
        &self.events
    }

    /// Waits asynchronously for the final execution report.
    ///
    /// If the event stream has not been consumed, it is disconnected before waiting so raw
    /// callback events cannot accumulate while the caller only waits for the report.
    pub async fn wait(self) -> Result<ExecutionReport> {
        let Self { completion, events } = self;
        drop(events);
        WaitExecution { completion }.await
    }
}

pub(crate) struct CompletionState {
    inner: Mutex<CompletionInner>,
}

struct CompletionInner {
    result: Option<Result<ExecutionReport>>,
    waker: Option<Waker>,
}

impl CompletionState {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(CompletionInner {
                result: None,
                waker: None,
            }),
        })
    }

    pub(crate) fn complete(&self, result: Result<ExecutionReport>) {
        let mut inner = self.inner.lock().unwrap();
        inner.result = Some(result);
        if let Some(waker) = inner.waker.take() {
            waker.wake();
        }
    }
}

struct WaitExecution {
    completion: Arc<CompletionState>,
}

impl Future for WaitExecution {
    type Output = Result<ExecutionReport>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut inner = self.completion.inner.lock().unwrap();
        if let Some(result) = inner.result.take() {
            Poll::Ready(result)
        } else {
            inner.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
