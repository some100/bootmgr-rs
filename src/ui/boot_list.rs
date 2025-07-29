//! Provides [`BootList`], which is a way to display [`Config`]s using ratatui.
//!
//! It may be constructed from an iterator of [`Config`]s, or through its new method from a [`BootMgr`] containing
//! a [`Vec`] of [`Config`]s.

use alloc::{string::String, vec::Vec};
use ratatui_widgets::list::ListState;

use crate::{boot::BootMgr, config::Config};

/// The UI frontend for the [`Config`]s.
pub struct BootList {
    /// The names or titles of the boot options.
    pub items: Vec<String>,

    /// The internal state of the boot options.
    pub state: ListState,
}

impl FromIterator<Config> for BootList {
    fn from_iter<I: IntoIterator<Item = Config>>(iter: I) -> Self {
        let items = iter
            .into_iter()
            .map(|config| config.title.unwrap_or(config.filename)) // if title is nonexistent, use the filename
            .collect();
        let state = ListState::default();
        Self { items, state }
    }
}

impl BootList {
    /// Creates a new [`BootList`] given a [`BootMgr`].
    ///
    /// This simply creates a [`BootList`] from the inner [`Vec<Config>`] of the [`BootMgr`],
    /// then selects the default option given from the [`BootMgr`].
    #[must_use = "Has no effect if the result is unused"]
    pub fn new(boot_mgr: &BootMgr) -> Self {
        let mut boot_list = Self::from_iter(boot_mgr.list());
        boot_list.state.select(Some(boot_mgr.get_default()));
        boot_list
    }
}
