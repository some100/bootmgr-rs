//! The configuration editor.

use alloc::{borrow::ToOwned, string::String};
use smallvec::SmallVec;

use crate::config::{Config, builder::ConfigBuilder};

/// The editor for [`Config`]s.
///
/// An example of how to use this editor in a frontend can be found in `bootmgr-rs-ratatui`. This is intended
/// to be a standardized interface for frontends to edit the configurations.
#[derive(Default)]
pub struct ConfigEditor {
    /// Stores which field is currently being edited.
    idx: usize,

    /// Stores the editable fields that are in the [`Config`].
    fields: SmallVec<[(&'static str, String); 8]>,
}

impl ConfigEditor {
    /// Create a new instance of [`ConfigEditor`].
    #[must_use = "Has no effect if the result is unused"]
    pub fn new(config: &Config) -> Self {
        let fields = config
            .get_str_fields()
            .map(|(k, v)| (k, v.cloned().unwrap_or_default()))
            .collect();
        Self { idx: 0, fields }
    }

    /// Update the selected field at idx.
    pub fn update_selected(&mut self, input: &str) {
        input.clone_into(&mut self.fields[self.idx].1);
    }

    /// Get the current index.
    #[must_use = "Has no effect if the result is unused"]
    pub fn idx(&self) -> usize {
        self.idx
    }

    /// Get a reference to the fields
    #[must_use = "Has no effect if the result is unused"]
    pub fn fields(&self) -> &[(&'static str, String)] {
        &self.fields
    }

    /// Move to the previous field of the editor.
    pub fn prev_field(&mut self) {
        if self.idx > 0 {
            self.idx -= 1;
        }
    }

    /// Move to the next field of the editor.
    pub fn next_field(&mut self) {
        if self.idx + 1 < self.fields.len() {
            self.idx += 1;
        }
    }

    /// Move to a named field of the editor. It will return true if the field actually exists.
    pub fn go_to_field(&mut self, name: &str) -> bool {
        match self.fields.iter().position(|x| x.0 == name) {
            Some(idx) => {
                self.idx = idx;
                true
            }
            None => false,
        }
    }

    /// Get the name of the current field
    #[must_use = "Has no effect if the result is unused"]
    pub fn current_name(&self) -> &'static str {
        self.fields[self.idx].0
    }

    /// Get the value of the current field.
    #[must_use = "Has no effect if the result is unused"]
    pub fn current_field(&self) -> &str {
        &self.fields[self.idx].1
    }

    /// Get the character count of the current field.
    #[must_use = "Has no effect if the result is unused"]
    pub fn chars(&self) -> usize {
        self.fields[self.idx].1.chars().count()
    }

    /// Build the [`ConfigEditor`] into a [`Config`] given the previous [`Config`].
    pub fn build(&self, config: &mut Config) {
        let builder =
            self.fields
                .iter()
                .fold(ConfigBuilder::from(&*config), |builder, (key, val)| {
                    if val.trim().is_empty() {
                        builder
                    } else {
                        match *key {
                            "title" => builder.title(val),
                            "version" => builder.version(val),
                            "machine_id" => builder.machine_id(val),
                            "sort_key" => builder.sort_key(val),
                            "options" => builder.options(val),
                            "devicetree" => builder.devicetree_path(val),
                            "architecture" => builder.architecture(val),
                            "efi" => builder.efi_path(val),
                            _ => builder,
                        }
                    }
                });
        *config = builder.build();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_editing() {
        let mut config = ConfigBuilder::new("foo.bar", ".bar")
            .efi_path("\\some\\path")
            .title("some title")
            .machine_id("12345678901234567890abcdef123456")
            .sort_key("some-sort-key")
            .build();
        let mut editor = ConfigEditor::new(&config);
        assert_eq!(editor.current_field(), "some title");
        editor.update_selected("this is another title");
        assert_eq!(editor.current_field(), "this is another title");
        editor.next_field();
        assert!(editor.current_field().is_empty());
        editor.update_selected("some version");
        assert_eq!(editor.current_field(), "some version");
        editor.prev_field();
        editor.update_selected("a different title");
        editor.build(&mut config);
        assert_eq!(config.title, Some("a different title".to_owned()));
        assert_eq!(config.version, Some("some version".to_owned()));
        assert!(config.options.is_none());
    }

    #[test]
    fn test_validation() {
        let mut config = ConfigBuilder::new("foo.bar", ".bar")
            .machine_id("12345678901234567890abcdef123456")
            .sort_key("some-sort-key")
            .build();
        let mut editor = ConfigEditor::new(&config);
        assert!(editor.go_to_field("machine_id"));
        editor.update_selected("a");
        assert!(editor.go_to_field("sort_key"));
        editor.update_selected("i@+-n,,v//alid");
        editor.build(&mut config);
        assert!(config.machine_id.is_none());
        assert!(config.sort_key.is_none());
    }
}
