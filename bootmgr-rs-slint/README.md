# bootmgr-rs-slint

A GUI frontend for `bootmgr-rs-core` made using Slint. This creates a basic carousel where boot options can be cycled around and booted similarly but not quite like rEFInd.

The default icons are public domain icons from Wikimedia Commons. To swap these icons, you need to resize your PNG images to 64x64 (using imagemagick or some similar program), then replace the appropriate icon at `ui/icons`. Due to no_std limitations with image parsing (and for code simplicity purposes), these icons cannot be loaded at runtime. They must be present at compile time.

This frontend is missing an editor feature. Slint provides a `TextEdit` widget for this purpose, so it should be a bit simpler to implement than with ratatui.

![rEFInd-ish carousel boot manager](/images/bootmgr-rs-slint.gif)