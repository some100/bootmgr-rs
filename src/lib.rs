#![no_std]

pub mod app;
pub mod boot;
pub mod config;
pub mod error;
pub mod features;
pub mod system;
pub mod ui;

#[cfg(feature = "editor")]
pub mod editor;

extern crate alloc;
