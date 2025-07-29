# bootmgr-rs

Windows BOOTMGR and systemd-boot inspired boot manager written in Rust

## Usage

Compile it, then install the produced .efi file to \EFI\BOOT\BOOTx64.efi. This includes support for macOS, Windows, UKIs, and BLS configuration files which are detected at runtime.

## Compilation

bootmgr-rs is written in Rust, so the Cargo toolchain is required for compilation.
```
git clone https://github.com/some100/bootmgr-rs
cargo build -r --target x86_64-unknown-uefi
```

## Testing

Tests can be run using `cargo test --lib` as usual. 

Integration tests are located in the `bootmgr-rs-tests` directory, and are built as a standard UEFI rust application. It can be built using `cargo build --target x86_64-unknown-uefi`.

## License

Licensed under the MIT License.

<img width="718" height="754" alt="image" src="https://github.com/user-attachments/assets/f36b905e-aee5-4a81-862d-d990ae464b35" />
