# Changelog

All notable changes to this project will be documented in this file.

## bootmgr-rs-core - [0.10.0] - 2025-08-11

### Changed

- Tighten visibility on multiple members. (BREAKING)

## bootmgr-rs-ratatui - [0.4.0] - 2025-08-11

### Changed

- Const-ified `Theme::new`.
- Internal code improvements.

### Removed

- Removed redundant `App::close` method.

## bootmgr-rs-slint - [0.5.0] - 2025-08-11

### Changed

- Const-ified a few functions that were eligible.
- Cursor is no longer directly drawn to framebuffer, but now directly uses GOP BLT.
- Internal code improvements and optimizations.

## bootmgr-rs-core - [0.9.0] - 2025-08-11

### Changed

- Const-ified many functions that were eligible.
- Internal code improvements.
- Clarify current MSRV in crate-level documentation.

## bootmgr-rs-core - [0.8.0] - 2025-08-10

### Added

- `locate_protocol` helper for getting a protocol.

### Changed

- Mark "bad" entries when getting preferred title.

## bootmgr-rs-core - [0.7.0] - 2025-08-10

### Changed

- Uses of `usize::MAX` replaced with safer `ONE_GIGABYTE` constant.
- Improved documentation.

### Removed

- `SecureBootError::AlreadyInstalled` error variant removed.

## bootmgr-rs-slint - [0.4.0] - 2025-08-10

### Added

- Added proper-er error handling (errors are now displayed as a popup).

### Changed

- `App::run()` now consumes `self` rather than a mutable reference to `self`.

## bootmgr-rs-slint - [0.3.0] - 2025-08-10

### Changed

- Improved documentation.
- Slint `listIdx` is now set exclusively by Rust.

## bootmgr-rs-ratatui - [0.3.0] - 2025-08-10

### Added

- New indicator for bad boot entries.

## bootmgr-rs-core - [0.6.0] - 2025-08-10

### Added

- More error variants added to `FsError`.

### Changed

- Improved documentation.
- `BootConfig` file errors are now propagated, unless the error was file not found.
- `UefiFileSystem` methods now return `Result<T, FsError>`. (BREAKING)

## bootmgr-rs-core - [0.5.0] - 2025-08-10

### Added

- Consolidated filesystem helpers into `UefiFileSystem` struct.

### Changed

- Improved documentation.
- Internal helper `slice_to_maybe_uninit` is now generic.

### Removed

- Removed almost every standalone filesystem helper. (BREAKING)

## bootmgr-rs-slint - [0.2.1] - 2025-08-09

### Changed

- Slightly improved program flow.
- Sleep when there are no active animations to save cycles.

## bootmgr-rs-minimal - [0.2.0] - 2025-08-08

### Changed

- Use boxed errors instead of anyhow, to remove a dependency.
- Remove unnecessary documentation.

## bootmgr-rs-ratatui - [0.2.0] - 2025-08-08

### Added

- Persistent configuration overlay.
- Allow removing persistent configurations.
- Use `ConfigEditor` instead of own editor implementation.

### Changed

- Editor state is now tracked with an enum instead of bools.

## bootmgr-rs-slint - [0.2.0] - 2025-08-08

### Added

- Added cursor/mouse support.

### Changed

- Greatly improved structuring and main loop.
- Adjust animation timings.

## bootmgr-rs-core - [0.4.0] - 2025-07-08

### Added

- `ConfigEditor` which exposes an available set of fields to the frontend, which can be edited and rebuilt into a `Config`.

### Changed

- Improved documentation.

### Fixed

- Bug with `boot/loader/efi.rs` where load options would be set for all images (even if empty).

## bootmgr-rs-core - [0.3.0] - 2025-06-08

### Added

- Added `get_preferred_title` method to `Config`, which is now the preferred way of getting a title.
- `HACKING.md` guide for an overview of the project.

### Changed

- Use Shim to load drivers instead of the default UEFI LoadImage.
- Privatize `install_security_override` and `uninstall_security_override`.

## bootmgr-rs-slint - [0.1.0] - 2025-06-08

### Added

- Slint UI frontend, a carousel graphical frontend to `bootmgr-rs-core`.
- Images of this frontend.

## bootmgr-rs-core - [0.2.0] - 2025-05-08

### Added

- Property testing for some unit tests.

### Changed

- Privatize many once public members of `bootmgr-rs-core` API. (BREAKING)
- Tighter lints for clippy and cargo.
- Update naming of some fields.

## bootmgr-rs-minimal - [0.1.0] - 2025-05-02

### Changed

- `bootmgr-rs-basic` renamed to `bootmgr-rs-minimal`.

## [0.5.0] - 2025-01-08

### Added

- `bootmgr-rs-basic` crate, which is a skeleton frontend for `bootmgr-rs-core`.
- `bootmgr-rs-core` crate, which is the UEFI logic separated from `bootmgr-rs`.
- `bootmgr-rs` now simply holds the ratatui frontend for `bootmgr-rs-core`.

### Changed

- Formatting.
- Improved documentation.

## [0.4.1] - 2025-01-08

### Changed

- Formatting.
- General code structure improvements.
- Use anyhow instead of `BootResult` in tests.
- Improved documentation.

### Fixed

- Bug with fuzzing that prevented it from working on release profile with lto enabled.

## [0.4.0] - 2025-01-08

### Added

- New `SecurityOverrideGuard` struct that automatically manages security override installation and uninstallation.
- Separate `secure_boot.rs` into `security_hooks.rs`.

### Changed

- Formatting.
- `reset_to_firmware` now returns `!` instead of `BootResult<!>`.
- Improved documentation.

## [0.3.3] - 2025-31-07

### Added

- `read_into` function which reads into a byte buffer instead of returning an allocated `Vec`.

### Changed

- Use `read_into` function in more places.

### Fixed

- Fixed an issue with xtasks where the release flag was not applying.

## [0.3.2] - 2025-31-07

### Changed

- Store a static slice inside of the Devicetree struct, and initialize it once on `Devicetree::new`.
- `DevicetreeFixup::fixup` is now safe due to no longer taking raw pointers as parameters.

## [0.3.1] - 2025-31-07

### Changed

- Improved documentation.
- Use safer `RefCell` instead of `UnsafeCell` for LoadOptions.
- Use `Cell` instead of `OnceCell` for security override.
- Removed even more usages of `unwrap`.

## [0.3.0] - 2025-31-07

### Added

- `xtask` build system.
- Use cargo workspace instead of one crate.
- Architecture validation for devicetree installation.
- SmallVec usage in some places instead of Vec.
- New internal helper `slice_to_maybe_uninit`.

### Changed

- Fuzzers now fuzz with raw bytes on `BootConfig` and BLS parsers.
- Crackdown on `unwrap` usage.

### Removed

- `read_to_string` function (use `str::from_utf8(read())) instead`).

## [0.2.1] - 2025-29-07

### Changed

- Improved documentation.
- Use `BootResult<T>` more extensively.
- Improved testing.

## [0.2.0] - 2025-28-07

### Added

- `bootmgr-rs-test` (integration testing suite).
- `BootConfig` fuzzer.
- Simple EFI over TFTP/PXE booting support.
- Security override installation and Shim support.
- Newtypes for validated `Config` fields.
- `shutdown` and `reset` functions.

### Changed

- Consolidated `Result<T, BootError>` into `BootResult<T>`.
- Changed boolean-based state into enum-based state in `App`.
- Improve documentation.
- Move `BootConfig` file to `\loader\bootmgr-rs.conf`.
- Use `DevicetreeGuard` for internally handling devicetree state.
- Add type-based validation for some fields of `Config`.
- General restructuring.
