//! Provides [`BootList`], which is a way to display [`Config`]s using ratatui.
//!
//! It may be constructed from an iterator of [`Config`]s, or through its new method from a [`BootMgr`] containing
//! a [`Vec`] of [`Config`]s.

use alloc::{string::{String, ToString}, vec::Vec};
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
            .enumerate()
            .map(|(i, config)| choose_title(config, i))
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

/// Picks a title for a [`Config`] using one of three sources.
/// 
/// If the title of the [`Config`] is found, then that is used and preferred because it indicates the preferred
/// name for the boot option.
/// If the title is not present, and the filename is not empty, then the filename is used. This is because it can
/// still indicate the source of a particular boot option or its origin.
/// If the filename is empty, then the index of the boot option is used. This is because at least some way of differentiating
/// the boot option from other boot options is required.
fn choose_title(config: Config, i: usize) -> String {
    config.title.unwrap_or_else(|| {
        if config.filename.is_empty() {
            i.to_string()
        } else {
            config.filename
        }
    })
}