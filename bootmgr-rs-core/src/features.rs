//! Stubs for features that are disabled

/// Creates an optional config.
///
/// For a config parser that implements `ConfigParser`, one should add the parser to this features file
/// in order to allow it to be disabled or enabled through the features.
///
/// This macro takes three parameters. The first of these is the feature as a string literal. This means
/// that whatever your feature is called (such as bls), it should be wrapped in double quotes as though
/// it were a string literal, like "bls".
/// The second is the feature as an identifier, and should be the name of the configuration parser module.
/// This should be the same as the previous parameter, without double quotes as it is not a string literal.
/// The third is the name of the config struct that implements `ConfigParser`. This can be named something like
/// `BlsConfig`.
/// The final macro invocation should look something like `optional_config!("bls", bls, BlsConfig)`.
macro_rules! optional_config {
    ($feature:literal, $name:ident, $config:ident) => {
        /// The parser for $config
        #[cfg(feature = $feature)]
        pub(crate) mod $name {
            pub(crate) use crate::config::parsers::$name::$config;
        }

        /// The disabled parser for $config
        #[cfg(not(feature = $feature))]
        pub(crate) mod $name {
            use crate::{
                config::{Config, parsers::ConfigParser},
                system::fs::UefiFileSystem,
            };
            use alloc::vec::Vec;

            pub(crate) struct $config;

            impl ConfigParser for $config {
                fn parse_configs(
                    _fs: &mut UefiFileSystem,
                    _handle: Handle,
                    _configs: &mut Vec<Config>,
                ) {
                }
            }
        }
    };
}

optional_config!("bls", bls, BlsConfig);
optional_config!("fallback", fallback, FallbackConfig);
optional_config!("osx", osx, OsxConfig);
optional_config!("shell", shell, ShellConfig);
optional_config!("uki", uki, UkiConfig);
optional_config!("windows", windows, WinConfig);
