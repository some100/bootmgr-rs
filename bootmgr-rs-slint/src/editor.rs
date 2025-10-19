// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! The optional basic editor for modifying [`Config`]s.

use alloc::vec::Vec;

use bootmgr::config::{Config, editor::ConfigEditor};
use slint::{Model, ModelRc, SharedString, ToSharedString};

/// The basic editor
#[derive(Default)]
pub struct Editor(ConfigEditor);

impl Editor {
    /// Creates a new [`Editor`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Load an editor from a config.
    pub fn load_config(&mut self, config: &Config) {
        self.0 = ConfigEditor::new(config);
    }

    /// Save an editor to a config.
    pub fn save_config(
        &mut self,
        config: &mut Config,
        fields: &ModelRc<(SharedString, SharedString)>,
    ) {
        self.save_fields(fields);
        self.0.build(config);
    }

    /// Get the fields of the config.
    pub fn get_fields(&self) -> ModelRc<(SharedString, SharedString)> {
        let fields: Vec<_> = self
            .0
            .fields()
            .iter()
            .map(|(x, y)| (x.to_shared_string(), y.to_shared_string()))
            .collect();

        ModelRc::from(&*fields)
    }

    /// Save the fields to the config.
    pub fn save_fields(&mut self, fields: &ModelRc<(SharedString, SharedString)>) {
        for (label, value) in fields.iter() {
            if self.0.go_to_field(&label) {
                self.0.update_selected(&value);
            }
        }
    }
}
