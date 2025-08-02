#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = bootmgr_rs_core::config::parsers::uki::UkiConfig::new(data);
});
