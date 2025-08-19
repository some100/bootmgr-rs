// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! The optional basic editor for modifying [`Config`]s.

use alloc::vec::Vec;
use bootmgr::config::{Config, editor::ConfigEditor};
use slint::{Model, ModelRc, SharedString, ToSharedString};

/// The basic editor
#[derive(Default)]
pub struct Editor {
    /// The [`ConfigEditor`].
    pub edit: ConfigEditor,
}

impl Editor {
    /// Creates a new [`Editor`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Load an editor from a config.
    pub fn load_config(&mut self, config: &Config) {
        self.edit = ConfigEditor::new(config);
    }

    /// Save an editor to a config.
    pub fn save_config(
        &mut self,
        config: &mut Config,
        fields: &ModelRc<(SharedString, SharedString)>,
    ) {
        self.save_fields(fields);
        self.edit.build(config);
    }

    /// Get the fields of the config.
    pub fn get_fields(&self) -> ModelRc<(SharedString, SharedString)> {
        let fields: Vec<_> = self
            .edit
            .fields()
            .iter()
            .map(|(x, y)| (x.to_shared_string(), y.to_shared_string()))
            .collect();

        ModelRc::from(&*fields)
    }

    /// Save the fields to the config.
    pub fn save_fields(&mut self, fields: &ModelRc<(SharedString, SharedString)>) {
        for (label, value) in fields.iter() {
            if self.edit.go_to_field(&label) {
                self.edit.update_selected(&value);
            }
        }
    }
}
