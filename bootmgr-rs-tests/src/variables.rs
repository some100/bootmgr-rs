// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

use bootmgr::{
    BootResult,
    system::variable::{get_variable, get_variable_str, set_variable, set_variable_str},
};
use uefi::{CStr16, cstr16, println};

use crate::press_for_reboot;

const VARIABLE_NAME: &CStr16 = cstr16!("TestVariable");
const VARIABLE_CONTENT: usize = 23;
const UPDATED_VARIABLE_CONTENT: usize = 24;
const STRING_VARIABLE_NAME: &CStr16 = cstr16!("TestStringVariable");
const STRING_VARIABLE_CONTENT: &CStr16 = cstr16!("foo");
const UPDATED_STRING_VARIABLE_CONTENT: &CStr16 = cstr16!("bar");

/// Check if the variables persisted.
///
/// # Panics
///
/// May panic if any of the assertions fail.
///
/// # Errors
///
/// May return an `Error` if the variables could not be set.
pub fn check_variable() -> BootResult<()> {
    if let Ok(num) = get_variable::<usize>(VARIABLE_NAME, None)
        && num != 0
        && let Ok(str) = get_variable_str(STRING_VARIABLE_NAME, None)
        && str == cstr16!("foo")
    {
        assert_ne!(num, UPDATED_VARIABLE_CONTENT);
        assert_ne!(str, UPDATED_STRING_VARIABLE_CONTENT);
        if num == VARIABLE_CONTENT {
            println!("Successfully got value of {VARIABLE_NAME}: {num}");
            println!("Successfully got value of {STRING_VARIABLE_NAME}: {str}");
            set_variable::<usize>(VARIABLE_NAME, None, None, Some(UPDATED_VARIABLE_CONTENT))?;
            set_variable_str(
                STRING_VARIABLE_NAME,
                None,
                None,
                Some(UPDATED_STRING_VARIABLE_CONTENT),
            )?;

            println!("Now testing if variable can be deleted");
            println!("A panic will result on reboot if it fails");
            set_variable::<usize>(VARIABLE_NAME, None, None, None)?;
            set_variable_str(STRING_VARIABLE_NAME, None, None, None)?;
            println!("Press a key to reboot");
            press_for_reboot();
        }
    }
    Ok(())
}

/// Test setting the values of the variables.
///
/// # Panics
///
/// May panic if any of the assertions fail.
///
/// # Errors
///
/// May return an `Error` if the variables could not be set.
pub fn test_variables() -> BootResult<()> {
    set_variable(VARIABLE_NAME, None, None, Some(VARIABLE_CONTENT))?;
    set_variable_str(
        STRING_VARIABLE_NAME,
        None,
        None,
        Some(STRING_VARIABLE_CONTENT),
    )?;
    println!("Set value of {VARIABLE_NAME} to {VARIABLE_CONTENT}");
    println!("Set value of {STRING_VARIABLE_NAME} to {STRING_VARIABLE_CONTENT}");
    assert_eq!(
        get_variable::<usize>(VARIABLE_NAME, None)?,
        VARIABLE_CONTENT
    );
    assert_eq!(
        get_variable_str(STRING_VARIABLE_NAME, None)?,
        STRING_VARIABLE_CONTENT
    );
    println!("Will now test if variable persists");
    println!("Press a key to reboot");
    press_for_reboot();
}
