extern crate winapi;
extern crate user32;
extern crate kernel32;
extern crate libc;
use std::ffi::OsStr;
use std::io::Error;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use std::mem;
use self::winapi::winuser::{WNDCLASSW,WNDPROC,CS_HREDRAW,CS_VREDRAW,WS_OVERLAPPEDWINDOW,CW_USEDEFAULT,MSG,WS_VISIBLE,PM_REMOVE};
use self::winapi::windef::{HWND,HICON,HCURSOR,HBRUSH,HMENU};
use self::winapi::minwindef::{UINT,WPARAM,LPARAM,LRESULT,HINSTANCE,LPVOID};

pub fn main() {
    // let wide: Vec<u16> = OsStr::new("Oi oiiii").encode_wide().chain(once(0)).collect();
    // let ret = unsafe {
    //     user32::MessageBoxW(null_mut(), wide.as_ptr(), wide.as_ptr(), winapi::MB_OK | winapi::MB_ICONINFORMATION)
    // };
    // if ret == 0 {
    //     println!("Failed: {:?}", Error::last_os_error());
    // }
    let windowClassName = to_wide_string("RustHeroWindowClass").as_ptr();

    let hInstance = unsafe {
        kernel32::GetModuleHandleW(0 as winapi::winnt::LPCWSTR)
    };

    let mut window = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(winproc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hInstance,
        hIcon: unsafe { user32::LoadIconW(hInstance, winapi::winuser::IDI_APPLICATION) },
        hCursor: unsafe { user32::LoadCursorW(hInstance, winapi::winuser::IDI_APPLICATION) },
        hbrBackground: 0 as HBRUSH,
        lpszMenuName: to_wide_string("RustHeroMenu").as_ptr(),
        lpszClassName: windowClassName
    };

    let ret = unsafe {
        user32::RegisterClassW(&mut window)
    };
    
    println!("Register class return value = {}", ret);

    let hwnd = unsafe {
        user32::CreateWindowExW(
            0,
            windowClassName,
            to_wide_string("FUCK").as_ptr(),
            WS_VISIBLE|WS_OVERLAPPEDWINDOW, //Style?
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0 as HWND,  //HWND? Optional?
            0 as HMENU,
            hInstance,
            0 as LPVOID
        )
    };
    

    loop {
        unsafe {
            let mut msg: MSG = mem::uninitialized();
            let res = user32::PeekMessageW(&mut msg, hwnd,0,0,PM_REMOVE);
            if res > 0 {
                user32::TranslateMessageW(&mut msg);
                user32::DispatchMessageW(&mut msg);
            }
        }
    }
    
}

fn to_wide_string(str: &str) -> Vec<u16> {
    OsStr::new(str).encode_wide().chain(once(0)).collect()
}


pub unsafe extern "system" fn winproc(h_wnd :HWND, 
	msg :UINT, w_param :WPARAM, l_param :LPARAM) -> LRESULT {
    if msg == winapi::winuser::WM_DESTROY {
    }
    return user32::DefWindowProcW(h_wnd, msg, w_param, l_param);
}