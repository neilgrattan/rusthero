extern crate winapi;
extern crate user32;
use std::ffi::OsStr;
use std::io::Error;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;

pub fn print_message(msg: &str) {
    let wide: Vec<u16> = OsStr::new(msg).encode_wide().chain(once(0)).collect();
    let ret = unsafe {
        user32::MessageBoxW(null_mut(), wide.as_ptr(), wide.as_ptr(), winapi::MB_OK)
    };
    if ret == 0 {
        println!("Failed: {:?}", Error::last_os_error());
    }
}