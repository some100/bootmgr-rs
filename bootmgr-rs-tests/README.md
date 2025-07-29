# bootmgr-rs-tests

Integration tests for bootmgr-rs.

This tests the custom actions (reboot, shutdown, reset to firmware), filesystem, and variable functionality.
Because regular integration tests are not available on UEFI, this is essentially a separate application that uses the library features of bootmgr-rs and tests them individually. This also tests parts of the "library" that cannot be unit tested, which includes the variables.
