#![cfg(all(not(fuzzing), not(test)))] // if fuzzing or testing on host, disable global alloc and use host alloc
use uefi::allocator::Allocator;

#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
