# bootmgr-rs-slint

A GUI demo for `bootmgr-rs-core` made using Slint. This creates a basic carousel where boot options can be cycled around and booted similarly but not quite like rEFInd.

The default icons are public domain icons from Wikimedia Commons. To swap these icons, you need to resize your PNG images to 64x64 (using imagemagick or some similar program), then replace the appropriate icon at `ui/icons`. Due to no_std limitations with image parsing, these icons cannot be loaded at runtime. They must be present at compile time.

Do note that this frontend, even if it is usable, is not yet as feature-complete as the ratatui frontend. The feature that it is missing is the `Config` editor. This should be somewhat trivial to implement as Slint already has a TextEdit widget.

![rEFInd-ish carousel boot manager](/images/bootmgr-rs-slint.gif)