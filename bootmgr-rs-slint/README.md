# bootmgr-rs-slint

A GUI demo for `bootmgr-rs-core` made using Slint. This creates a basic carousel where boot options can be cycled around and booted similarly but not quite like rEFInd.

The default icons are public domain icons from Wikimedia Commons. To swap these icons, you need to resize your PNG images to 64x64 (using imagemagick or some similar program), then replace the appropriate icon at `ui/icons`. Due to no_std limitations with image parsing, these icons cannot be loaded at runtime. They must be present at compile time.

Do note that this frontend, even if it is still usable, is not yet as feature-complete as the ratatui frontend.

This is especially true if doing developer work, since errors and panics are significantly less obvious with the GUI as opposed to using ratatui. A panic will simply cause a reboot on key press. However, those that have actual experience in graphical design may find it useful, as it provides a bundled in Slint backend as well as slint callbacks for interacting with the boot manager.

![rEFInd-ish carousel boot manager](/images/bootmgr-rs-slint.gif)