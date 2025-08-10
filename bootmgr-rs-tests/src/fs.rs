use bootmgr_rs_core::{BootResult, system::fs::UefiFileSystem};
use uefi::{CStr16, cstr16, println};

use crate::press_for_reboot;

const FILE_PATH: &CStr16 = cstr16!("\\foo.file");
const FILE_CONTENT: &[u8] = &55usize.to_le_bytes();
const ALT_FILE_PATH: &CStr16 = cstr16!("\\foo.other");

pub fn test_filesystem() -> BootResult<()> {
    let mut fs = UefiFileSystem::from_image_fs()?;

    fs.create(FILE_PATH)?;
    assert!(fs.exists(FILE_PATH));
    fs.write(FILE_PATH, FILE_CONTENT)?;
    assert_eq!(fs.read(FILE_PATH)?, FILE_CONTENT);
    fs.rename(FILE_PATH, ALT_FILE_PATH)?;
    assert!(!fs.exists(FILE_PATH));
    assert_eq!(fs.read(ALT_FILE_PATH)?, FILE_CONTENT);
    fs.delete(ALT_FILE_PATH)?;
    assert!(!fs.exists(ALT_FILE_PATH));
    println!("All filesystem assertions passed!");
    println!("Press a key to reboot");
    press_for_reboot();
}
