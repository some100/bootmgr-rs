//! Simple UEFI backend for the [`log`] crate.

use core::fmt::Write;

use alloc::boxed::Box;
use log::{Level, Metadata, Record};
use uefi::{runtime, system::with_stdout};

/// A simple logging backend for UEFI.
#[derive(Default)]
pub struct UefiLogger;

impl UefiLogger {
    /// Constructs a new [`UefiLogger`].
    #[must_use = "Has no effect if the result is unused"]
    pub const fn new() -> Self {
        Self
    }

    /// Constructs a new [`UefiLogger`], then immediately leaks it so that it can be used with `set_logger`.
    #[must_use = "Has no effect if the result is unused"]
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
            let level = record.level();
            let file = record.file().unwrap_or_default();
            let line = record.line().unwrap_or_default();
            let args = record.args();
            with_stdout(|stdout| {
                let _ = stdout.write_fmt(format_args!("[{time} {level} {file}:{line}] - {args}\n"));
            });
        }
    }

    fn flush(&self) {}
}
