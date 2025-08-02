use bootmgr_rs_core::{
    BootResult,
    system::variable::{get_variable, set_variable},
};
use uefi::{CStr16, cstr16, println};

use crate::press_for_reboot;

const VARIABLE_NAME: &CStr16 = cstr16!("TestVariable");
const VARIABLE_CONTENT: usize = 23;
const UPDATED_VARIABLE_CONTENT: usize = 24;

pub fn check_variable() -> BootResult<()> {
    if let Ok(num) = get_variable::<usize>(VARIABLE_NAME, None)
        && num != 0
    {
        assert_ne!(num, UPDATED_VARIABLE_CONTENT);
        if num == VARIABLE_CONTENT {
            println!("Successfully got value of TestVariable: {num}");
            set_variable::<usize>(VARIABLE_NAME, None, None, Some(UPDATED_VARIABLE_CONTENT))?;

            println!("Now testing if variable can be deleted");
            println!("A panic will result on reboot if it fails");
            set_variable::<usize>(VARIABLE_NAME, None, None, None)?;
            println!("Press a key to reboot");
            press_for_reboot();
        }
    }
    Ok(())
}

pub fn test_variables() -> BootResult<()> {
    set_variable(VARIABLE_NAME, None, None, Some(VARIABLE_CONTENT))?;
    println!("Set value of TestVariable to 23");
    assert_eq!(
        get_variable::<usize>(VARIABLE_NAME, None)?,
        VARIABLE_CONTENT
    );
    println!("Will now test if variable persists");
    println!("Press a key to reboot");
    press_for_reboot();
}
