// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: GPL-2.0-or-later

//! A parser for the Windows BCD and Windows boot manager.

#[cfg(feature = "windows_bcd")]
pub mod windows_bcd;

#[cfg(feature = "windows_bcd")]
pub use windows_bcd::WinConfig;

#[cfg(not(feature = "windows_bcd"))]
pub mod windows_auto;

#[cfg(not(feature = "windows_bcd"))]
pub use windows_auto::WinConfig;