#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = bootmgr_rs::config::parsers::windows::WinConfig::new(data);
});
