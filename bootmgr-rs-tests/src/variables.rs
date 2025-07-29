use bootmgr_rs::{boot::action::reboot, system::variable::{get_variable, set_variable}};
use uefi::{cstr16, println, CStr16};

use crate::read_key;

const VARIABLE_NAME: &CStr16 = cstr16!("TestVariable");
const VARIABLE_CONTENT: usize = 23;
const UPDATED_VARIABLE_CONTENT: usize = 24;

pub fn check_variable() {
    if let Ok(num) = get_variable::<usize>(VARIABLE_NAME, None) &&
        num != 0
    {
        assert_ne!(num, UPDATED_VARIABLE_CONTENT);
        if num == VARIABLE_CONTENT {
            println!("Successfully got value of TestVariable: {num}");
            set_variable::<usize>(VARIABLE_NAME, None, None, Some(UPDATED_VARIABLE_CONTENT)).unwrap();

            println!("Now testing if variable can be deleted");
            println!("A panic will result on reboot if it fails");
            set_variable::<usize>(VARIABLE_NAME, None, None, None).unwrap();
            println!("Press a key to reboot");
            let _ = read_key();
            reboot::reset();
        }
    }
}

pub fn test_variables() {
    set_variable(VARIABLE_NAME, None, None, Some(VARIABLE_CONTENT)).unwrap();
    println!("Set value of TestVariable to 23");
    assert_eq!(get_variable::<usize>(VARIABLE_NAME, None).unwrap(), VARIABLE_CONTENT);
    println!("Will now test if variable persists");
    println!("Press a key to reboot");
    let _ = read_key();
    reboot::reset();
}