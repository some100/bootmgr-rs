#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = bootmgr_rs::config::parsers::bls::BlsConfig::new(&s);
    }
});
