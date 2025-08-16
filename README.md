# bootmgr-rs

UEFI-only boot manager library written in Rust

## Quickstart

```sh
git clone https://github.com/some100/bootmgr-rs
cd bootmgr-rs

# If rust is not installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

cargo xtask build -r

# For x64 systems
cp target/x86_64-unknown-uefi/release-lto/bootmgr-rs-ratatui.efi /boot/efi/EFI/BOOT/BOOTx64.efi

# For x86 systems
cp target/i686-unknown-uefi/release-lto/bootmgr-rs-ratatui.efi /boot/efi/EFI/BOOT/BOOTia32.efi

# For aarch64 systems
cp target/aarch64-unknown-uefi/release-lto/bootmgr-rs-ratatui.efi /boot/efi/EFI/BOOT/BOOTaa64.efi
```

## Usage

Compile it, then install the produced .efi file to \EFI\BOOT\BOOTx64.efi (or the appropriate name for your architecture). This includes support for macOS, Windows, UKIs, and BLS configuration files which are detected at runtime.

## Compilation

bootmgr-rs is written in Rust, so the Cargo toolchain is required for compilation.

The command must be ran at the root of the repository.
```sh
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

![systemd-boot and Windows bootmgr-like interface for a bootloader](/images/bootmgr-rs-ratatui.png)

Example frontend using ratatui.