use bootmgr_rs::system::fs::{check_file_exists, create, delete, read, rename, write};
use uefi::{boot, cstr16, println, CStr16};

const FILE_PATH: &CStr16 = cstr16!("\\foo.file");
const FILE_CONTENT: &[u8] = &55usize.to_le_bytes();
const ALT_FILE_PATH: &CStr16 = cstr16!("\\foo.other");

pub fn test_filesystem() {
    let mut fs = boot::get_image_file_system(boot::image_handle()).unwrap();
    create(&mut fs, FILE_PATH).unwrap();
    assert!(check_file_exists(&mut fs, FILE_PATH));
    write(&mut fs, FILE_PATH, FILE_CONTENT).unwrap();
    assert_eq!(read(&mut fs, FILE_PATH).unwrap(), FILE_CONTENT);
    rename(&mut fs, FILE_PATH, ALT_FILE_PATH).unwrap();
    assert!(!check_file_exists(&mut fs, FILE_PATH));
    assert_eq!(read(&mut fs, ALT_FILE_PATH).unwrap(), FILE_CONTENT);
    delete(&mut fs, ALT_FILE_PATH).unwrap();
    assert!(!check_file_exists(&mut fs, ALT_FILE_PATH));
    println!("All filesystem assertions passed!");
}