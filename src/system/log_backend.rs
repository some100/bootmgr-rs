//! UEFI backend for log

use core::fmt::Write;

use alloc::boxed::Box;
use log::{Level, Metadata, Record};
use uefi::{runtime, system::with_stdout};

#[derive(Default)]
pub struct UefiLogger;

impl UefiLogger {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    #[must_use]
    pub fn static_new() -> &'static Self {
        Box::leak(Box::new(Self::new()))
    }
}

impl log::Log for UefiLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let time = runtime::get_time().unwrap_or(runtime::Time::invalid());
            with_stdout(|stdout| {
                let _ = writeln!(
                    stdout,
                    "[{} {} {}:{}] - {}",
                    time,
                    record.level(),
                    record.file().unwrap_or_default(),
                    record.line().unwrap_or_default(),
                    record.args()
                );
            });
        }
    }

    fn flush(&self) {}
}
