# Overview

This is a boot manager library that comes with multiple frontends to that library. The primary way of building, testing, and fuzzing this library is through using `cargo xtask`. If you already have the Cargo toolchain installed, run these commands at the root:
```sh
git clone https://github.com/some100/bootmgr-rs
cd bootmgr-rs
cargo xtask build
```

If you would instead like to build a crate individually instead of the entire workspace, you can instead run this command:
```sh
# replace x86_64-unknown-uefi with whatever target
cargo build --target x86_64-unknown-uefi --features global_allocator,panic_handler
```

# Structure

The core crate is located at bootmgr-rs-core. The frontends will generally be named bootmgr-rs-\[frontend\]. The core crate is structured into 4 modules, those being:

* boot (provides `BootMgr` as well as boot-related things)
* config (provides `Config` and `ConfigBuilder`, houses parsers)
* error (provides `BootError`)
* system (provides UEFI, filesystem, and protocol helpers)

config contains parsers, which are essentially structs implementing `ConfigParser` that, given an `fs` and `handle`, will generate boot entries in the format of `Config`, that are pushed into the `configs` parameter.

boot itself is structured into 5 submodules, those being:

* action (provides firmware reset, reboot, and shutdown functions, as well as a PXE parser)
* config (provides `BootConfig`, which exposes settings for boot manager)
* devicetree (provides `install_devicetree`, which installs and fixups a devicetree blob)
* loader (provides EFI loader and EFI over TFTP loader)
* secure_boot (provides `SecurityOverrideGuard`, which may install security protocol overrides for Shim)

The general flow of a program with this crate is as follows:

1. The `BootMgr` struct is created. This will call `parse_all_configs`, which collects every available scanned `Config` into `BootMgr`, as well as special boot options.
2. The frontend will poll for inputs or have some other way of selecting the boot option.
3. Once a boot option is selected (through its index), the `load` method is called on the `BootMgr`.
    * This will call `boot::loader::load_boot_option` on the boot option's `Config`, which delegates to the `run` method of the `Config`'s `action` field.
    * Depending on the boot option, this will lead to the following loaders: If it is a special boot option, it will be reboot, shutdown, or reset to firmware. If the action indicates that it's a normal boot program (`BootAction::BootEfi`), then it will lead to the EFI loader being used. Otherwise, if it's a EFI over TFTP program (`BootAction::BootTftp`). For simplicity, this will focus on the EFI loader.
    * The handle is unwrapped from the `Config`'s `fs_handle`, and opened into a filesystem. This handle is how the filesystem from which the `Config` originates is tracked. Afterwards, the image specified by the `Config` through `efi_path` is converted into a `DevicePath`.
    * `shim_load_image` is then called, specifying the `DevicePath` as the source. Depending on if Shim is installed, this will either install the necessary security overrides, or load the image through UEFI as is.
    * The image returned is then setup, with devicetree installations, and LoadOptions being set as needed if specified.
    * Finally, the setup image is returned through `load_boot_option`, which is returned to the program.
4. The frontend will now break out of its poll loop, returning back into the entry function with the image `Handle`.
5. Finally, the image is started in the entry function. The reason why the image is not started in the `BootMgr`'s `load` method is to ensure that every protocol is properly dropped and closed before control is handed off to the next image.

# Writing a parser

In order to create a parser, it must first implement `ConfigParser`, which can be done by detecting if a file exists for example, then using the `ConfigBuilder` in order to create a `Config` that will then be pushed into the `configs` parameter. Then, add it as a module in `bootmgr-rs-core/src/config/parsers.rs` as well as to the `Parsers` enum. Afterwards, add it to `bootmgr-rs-core/src/features.rs` using the `optional_config!` macro, then add it as a feature flag in the `Cargo.toml`.

Optionally, you can also add an icon for it in `bootmgr-rs-slint`, as well as unit testing and fuzzing in xtask if applicable.

A good example of an auto-detecting "parser" can be found in `bootmgr-rs-core/src/config/parsers/shell.rs`. This simply checks for the existence of `\shellx64.efi`. It specifies the EFI path, the title, the sort key, the filesystem handle, as well as the origin of the `Config`, then pushes a built `Config` into the `configs` parameter. 

A good example of a parser that actually parses can be found in `bootmgr-rs-core/src/config/parsers/bls.rs`. This builds the `Config` based on information that was received while parsing the BLS configuration file. Because it actually parses things, it also has unit tests and fuzzing as well, to ensure the expected result.

# Writing a frontend

A simple example to start off of can be found in `bootmgr-rs-minimal`. This is a one-file simple boot manager that simply creates a `BootMgr`, print all the `Config`s inside of it, then loop until a number in range of the `BootMgr` list is pressed, then boot that respective entry. 

The overall structure of the frontends should remain very similar, as any protocols that your program may use (i.e `Input`, `Output`, `GraphicsOutput`) may remain while starting the image at the same time. 

By having essentially another main function that returns a `Handle` while the actual main function starts the image, every protocol will be properly closed before the image starts because they will go out of scope before the image is started. As long as this overall structure remains the same, it does not matter how your frontend is presented.

If the frontend library allows for it, you can also try implementing features specified in the `BootConfig`, such as an editor, or theming.

# Testing

The private library side of this codebase runs unit tests with the standard Rust test runner. To run clippy, unit tests, and doctests, run the command at the root:
```sh
cargo xtask test
```

In order to run integration tests (which test public parts of the library), run the command at the root:
```sh
cargo xtask test run
```

Of course, tests can also be ran without using xtask.
```sh
cargo clippy --features global_allocator,panic_handler -- -C panic=abort
cargo test --lib
cargo test --doc
```

Using xtask is still preferred, however.

# Style

You should always run `cargo fmt` and `cargo xtask test` after every significant change is made. As with most Rust projects, the code should aim to be idiomatic Rust. 

Usage of `clone` should be minimized, and used only if it is inexpensive to clone or if absolutely necessary and unlikely to impact performance. 

The primary exception to this of course is reference counting smart pointers like `Rc`. Another exception to this is when cloning filenames for error handling, if the error type takes an owned `String`. This is necessary if the content of the error type must be dynamic and changing.

A notable exception to this is `to_owned`, which might be necessary if you want to put an owned value in a struct but only have a reference. However, similarly to `clone`, it should not be overused, and for other cases you should look into seeing if a reference can be used in those situations instead.

Always prefer borrowed types as arguments, and owned types as return values. This is unless you are planning to consume the owned type in the argument (if you are cloning immediately after, you should be using a reference). 

`unwrap()` should never be used. If a certain call is infallible, using `expect("why this is infallible")` is a little bit more clear in intent than using unwrap. Always use `map_err()` instead, or at the very least `expect()` specifically for infallible operations.

Try to use `Result` types if possible instead of silently swallowing up errors, or panicking.

Every single unsafe block that is used must be preceded by a `// SAFETY:` comment that explains why it is safe, or in what situations it may be unsafe. This is enforced using the `unsafe_op_in_unsafe_fn` lint. In addition to this, every module must have a `Safety` section, listing every usage of unsafe in the module and why it is safe, or why it may be unsafe. Unsafe should only be used when strictly necessary. If a safe alternative can be used (i.e. `Cell`/`OnceCell`/`RefCell` over `UnsafeCell`), then it should be used.

Every module and item should be documented. Using `cargo xtask test` should fail in case a public item is undocumented, and warn in case a private item is undocumented.

Patterns such as `Rc<RefCell<T>>` are highly discouraged. This is a very easy way to enter the dreaded "type hell." Only do this if you are in a frontend and are practically forced to do this.

Generally speaking, you should not try to disable the clippy lints if possible. This is especially the case if there are obvious ways to resolve the lint without disabling it (for example, instead of casting with `as`, use `try_from` and `unwrap_or` with a default value). However, if a clippy lint simply cannot stay enabled (like with the slint frontend, where the autogenerated slint code produces false warns), then the clippy lint is fine to disable. Ensure that the reason is specified for this as well.

# Fuzzing

Fuzzing internally uses cargo-fuzz. Depending on the parser you want to fuzz, simply run one of the following commands:
```sh
# BLS Type #1 parser
cargo xtask fuzz bls

# Bootloader configuration parser
cargo xtask fuzz boot

# BLS Type #2 (UKI) parser
cargo xtask fuzz uki

# Windows BCD parser
cargo xtask fuzz win
```

Alternatively, using cargo-fuzz directly
```sh
cargo fuzz run bls

cargo fuzz run boot

cargo fuzz run uki

cargo fuzz run win
```

Seed corpuses should be placed in the directory fuzz/corpus/`<FUZZER>`, where `<FUZZER>` is whichever parser you are fuzzing. If an unexpected panic or otherwise interesting result was found, it will be located in fuzz/artifacts/`<FUZZER>`.

Once you find an interesting result, try to locate the origin of the result. If it was within this repository (for example, in the BLS parser), then an issue should be opened in this repo. If it was in a dependency crate, however (like nt-hive), then the issue should be opened within that respective crate's repository or bugtracker.
