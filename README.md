# bootmgr-rs

Windows BOOTMGR and systemd-boot inspired boot manager written in Rust

## Usage

Compile it, then install the produced .efi file to \EFI\BOOT\BOOTx64.efi. This includes support for macOS, Windows, UKIs, and BLS configuration files which are detected at runtime.

## Compilation

bootmgr-rs is written in Rust, so the Cargo toolchain is required for compilation.
```
git clone https://github.com/some100/bootmgr-rs
cargo xtask build -r
```

## Testing

Unit tests and clippy can be run using `cargo xtask test`.

Integration tests can be run using `cargo xtask test run`.

Fuzz tests can be run with `cargo xtask fuzz -f <PARSER>`, where PARSER is one of `bls`, `boot`, `uki`, and `win`.

If the main program needs to be tested, then it can be run using `cargo xtask run`.

## License

Licensed under the MIT License.
