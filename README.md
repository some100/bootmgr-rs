# bootmgr-rs

Windows BOOTMGR and systemd-boot inspired UEFI-only boot manager written in Rust

## Usage

Compile it, then install the produced .efi file to \EFI\BOOT\BOOTx64.efi. This includes support for macOS, Windows, UKIs, and BLS configuration files which are detected at runtime.

## Compilation

bootmgr-rs is written in Rust, so the Cargo toolchain is required for compilation.

The command must be ran at the root of the repository.
```
git clone https://github.com/some100/bootmgr-rs
cargo xtask build -r
```

Different compilation targets can be specified using `--target`. The currently supported and available targets are `x86_64-unknown-uefi`, `i686-unknown-uefi`, and `aarch64-unknown-uefi`.

## Testing

Unit tests and clippy can be run using `cargo xtask test`.

Integration tests can be run using `cargo xtask test run`.

If the main program needs to be tested, then it can be run using `cargo xtask run`.

## Fuzzing

Fuzzing tests are ran using `cargo xtask fuzz <PARSER>`, where `<PARSER>` is one of bls, boot, uki, and win. Alternatively, if cargo-fuzz is already installed, they can be ran using `cargo fuzz run <PARSER>`. Seed corpuses are provided in the directory `fuzz/corpus/<PARSER>`, and any interesting artifacts (such as panics) will be found in the directory `fuzz/artifacts/<PARSER>`.

## License

Licensed under the MIT License.
