# bootmgr-rs-slint

A GUI frontend for `bootmgr-rs-core` made using Slint. This creates a basic carousel where boot options can be cycled around and booted similarly but not quite like rEFInd.

The default icons are public domain icons from Wikimedia Commons. To swap these icons, you need to resize your PNG images to 64x64 (using imagemagick or some similar program), then replace the appropriate icon at `ui/icons`. Due to no_std limitations with image parsing (and for code simplicity purposes), these icons cannot be loaded at runtime. They must be present at compile time.

This frontend has an editor implemented using Slint's `LineEdit` widget. This editor is strictly mouse driven. Either ESC or the Cancel button can be used to exit the editor.

![rEFInd-ish carousel boot manager](/images/bootmgr-rs-slint.gif)