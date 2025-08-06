# bootmgr-rs-slint

A GUI demo for `bootmgr-rs-core` made using Slint. This creates a basic carousel where boot options can be cycled around and booted similarly but not quite like rEFInd.

This is still a demo (i'm not a ui designer), and the ratatui implementation should definitely be preferred over this if possible.

This is especially true if doing developer work, since errors and panics are significantly less obvious with the GUI as opposed to using ratatui.

![rEFInd-ish carousel boot manager](/images/bootmgr-rs-slint.gif)