use std::{
    sync::{Arc, Mutex, Weak},
    thread,
    time::Duration,
};

use crate::{
    ExecutionEnvironment, ExecutionEvent, ExecutionHandle, ExecutionReport, SessionFailure,
    SessionReport, SessionStatus,
    driver::CoreDriver,
    session::{RunnerConfig, SessionContext, SessionRunner},
};

#[derive(Default)]
pub(crate) struct FakeDriver {
    pub order: Mutex<Vec<String>>,
    pub reports: Mutex<Vec<SessionReport>>,
    self_ref: Mutex<Weak<FakeDriver>>,
}

impl FakeDriver {
    pub fn with_reports(reports: Vec<SessionReport>) -> Arc<Self> {
        let driver = Arc::new(Self {
            order: Mutex::new(Vec::new()),
            reports: Mutex::new(reports),
            self_ref: Mutex::new(Weak::new()),
        });
        *driver.self_ref.lock().unwrap() = Arc::downgrade(&driver);
        driver
    }
}

struct FakeSessionRunner {
    driver: Arc<FakeDriver>,
}

impl SessionRunner for FakeSessionRunner {
    fn run_session(&mut self, ctx: SessionContext) -> SessionReport {
        self.driver
            .order
            .lock()
            .unwrap()
            .push(format!("session:{}", ctx.session_index));
        self.driver.reports.lock().unwrap().remove(0)
    }
}

impl CoreDriver for FakeDriver {
    fn connect(&self, config: RunnerConfig) -> crate::Result<Box<dyn SessionRunner>> {
        let driver = self.self_ref.lock().unwrap().upgrade().unwrap();
        self.order.lock().unwrap().push(format!(
            "connect:{}:{}",
            config.address, config.execution.auto_reconnect
        ));
        config.events.send(ExecutionEvent::Connecting {
            address: config.address.clone(),
        });
        config.events.send(ExecutionEvent::Connected);
        if config.stop.is_requested() {
            return Ok(Box::new(FakeSessionRunner { driver }));
        }
        Ok(Box::new(FakeSessionRunner { driver }))
    }
}

pub(crate) struct FakeEnvironment {
    pub prepare_address: Option<String>,
    pub order: Arc<Mutex<Vec<String>>>,
}

impl ExecutionEnvironment for FakeEnvironment {
    type Error = std::io::Error;

    fn prepare(&mut self) -> std::result::Result<Option<String>, Self::Error> {
        self.order.lock().unwrap().push(String::from("prepare"));
        Ok(self.prepare_address.clone())
    }

    fn close(&mut self) -> std::result::Result<(), Self::Error> {
        self.order.lock().unwrap().push(String::from("close"));
        Ok(())
    }
}

pub fn report(status: SessionStatus, name: &str) -> SessionReport {
    SessionReport {
        name: Some(String::from(name)),
        status,
        failure: match status {
            SessionStatus::Succeeded => None,
            SessionStatus::Failed => Some(SessionFailure::new(crate::ErrorKind::Task, "boom")),
            SessionStatus::Interrupted => None,
        },
    }
}

pub fn block_on(handle: ExecutionHandle) -> crate::Result<ExecutionReport> {
    use std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
    };

    fn raw_waker() -> RawWaker {
        fn clone(_: *const ()) -> RawWaker {
            raw_waker()
        }
        fn wake(_: *const ()) {}
        fn wake_by_ref(_: *const ()) {}
        fn drop(_: *const ()) {}

        RawWaker::new(
            std::ptr::null(),
            &RawWakerVTable::new(clone, wake, wake_by_ref, drop),
        )
    }

    let waker = unsafe { Waker::from_raw(raw_waker()) };
    let mut future = Box::pin(handle.wait());

    loop {
        let mut context = Context::from_waker(&waker);
        if let Poll::Ready(result) = Pin::new(&mut future).poll(&mut context) {
            break result;
        }
        thread::sleep(Duration::from_millis(10));
    }
}
