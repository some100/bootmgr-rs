//! Simple stub to use UEFI allocator as the global allocator.
//!
//! This is enabled when the `global_allocator` feature is enabled, in case the user wanted to roll their
//! own custom allocator or for fuzzing/testing.

#![cfg(feature = "global_allocator")]
use uefi::allocator::Allocator;

/// The UEFI global allocator.
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
