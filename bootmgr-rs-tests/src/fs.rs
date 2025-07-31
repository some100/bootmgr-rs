use bootmgr_rs::{
    BootResult,
    system::fs::{check_file_exists, create, delete, read, rename, write},
};
use uefi::{CStr16, boot, cstr16, println};

use crate::press_for_reboot;

const FILE_PATH: &CStr16 = cstr16!("\\foo.file");
const FILE_CONTENT: &[u8] = &55usize.to_le_bytes();
const ALT_FILE_PATH: &CStr16 = cstr16!("\\foo.other");

pub fn test_filesystem() -> BootResult<()> {
    let mut fs = boot::get_image_file_system(boot::image_handle())?;
    create(&mut fs, FILE_PATH)?;
    assert!(check_file_exists(&mut fs, FILE_PATH));
    write(&mut fs, FILE_PATH, FILE_CONTENT)?;
    assert_eq!(read(&mut fs, FILE_PATH)?, FILE_CONTENT);
    rename(&mut fs, FILE_PATH, ALT_FILE_PATH)?;
    assert!(!check_file_exists(&mut fs, FILE_PATH));
    assert_eq!(read(&mut fs, ALT_FILE_PATH)?, FILE_CONTENT);
    delete(&mut fs, ALT_FILE_PATH)?;
    assert!(!check_file_exists(&mut fs, ALT_FILE_PATH));
    println!("All filesystem assertions passed!");
    println!("Press a key to reboot");
    press_for_reboot();
}
