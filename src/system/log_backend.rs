use core::fmt::Write;

use alloc::{boxed::Box, format};
use log::{Level, Metadata, Record};
use uefi::{runtime, system::with_stdout};

pub struct UefiLogger;

impl UefiLogger {
    pub fn new() -> Self {
        Self
    }

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
            let message = format!(
                "[{} {} {}:{}] - {}",
                time,
                record.level(),
                record.file().unwrap_or_default(),
                record.line().unwrap_or_default(),
                record.args()
            );
            with_stdout(|stdout| {
                let _ = stdout.write_str(&*message);
            });
        }
    }

    fn flush(&self) {}
}
