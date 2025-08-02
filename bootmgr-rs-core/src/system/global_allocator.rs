//! Simple stub to use UEFI allocator as the global allocator.
//!
//! This is disabled when fuzzing or testing, to avoid conflicts with std.

#![cfg(not(any(fuzzing, test, doctest)))] // if fuzzing or testing on host, disable global alloc and use host alloc
use uefi::allocator::Allocator;

/// The UEFI global allocator.
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
