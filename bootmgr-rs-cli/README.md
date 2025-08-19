# bootmgr-rs-minimal

A basic CLI frontend to `bootmgr`, which can be used as a command line program through the UEFI shell. Alternatively, you can also use this from the firmware boot manager by using a tool such as `efibootmgr`, then specifying boot options with `-u` or `--unicode`.

# Licensing

This frontend is licensed under the MIT license. However, with the `windows_bcd` feature enabled for `bootmgr`, the binary will be licensed under GPLv2 or later.