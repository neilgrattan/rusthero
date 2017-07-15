extern crate user32;
extern crate winapi;
extern crate kernel32;
extern crate libc;
use std::ffi::OsStr;
// use std::io::Error;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
// use std::ptr::null_mut;
use std::mem;
use self::winapi::winuser::{WNDCLASSW,CS_HREDRAW,CS_VREDRAW,WS_OVERLAPPEDWINDOW,CW_USEDEFAULT,MSG,WS_VISIBLE,PM_REMOVE,WM_PAINT,WM_SIZE,WM_CLOSE,WM_ACTIVATEAPP};
use self::winapi::windef::{HWND,HBRUSH,HMENU};
use self::winapi::minwindef::{UINT,WPARAM,LPARAM,LRESULT,LPVOID};

static mut GLOBAL_RUNNING: bool = true;

pub fn main() {

    let window_class_name = to_wide_string("RustHeroWindowClass").as_ptr();
    let h_instance = unsafe {
        kernel32::GetModuleHandleW(0 as winapi::winnt::LPCWSTR)
    };

    let mut window = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(winproc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: h_instance,
        hIcon: unsafe { user32::LoadIconW(h_instance, winapi::winuser::IDI_APPLICATION) },
        hCursor: unsafe { user32::LoadCursorW(h_instance, winapi::winuser::IDI_APPLICATION) },
        hbrBackground: 0 as HBRUSH,
        lpszMenuName: to_wide_string("RustHeroMenu").as_ptr(),
        lpszClassName: window_class_name
    };

    let ret = unsafe {
        user32::RegisterClassW(&mut window)
    };

    if ret == 0 {
        println!("Registering the window class failed.");
        return
    }
    
    let hwnd = unsafe {
        user32::CreateWindowExW(
            0,
            window_class_name,
            to_wide_string("Rust Hero").as_ptr(),
            WS_VISIBLE|WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0 as HWND,
            0 as HMENU,
            h_instance,
            0 as LPVOID
        )
    };
    
    if hwnd.is_null() {
        println!("Failed to create window.");
        return;
    }

    unsafe {
        while GLOBAL_RUNNING {
            let mut msg: MSG = mem::uninitialized();
            while user32::PeekMessageW(&mut msg, hwnd,0,0,PM_REMOVE) != 0 {
                user32::TranslateMessage(&mut msg);
                user32::DispatchMessageW(&mut msg);
            }
        }
    }
}

fn to_wide_string(str: &str) -> Vec<u16> {
    OsStr::new(str).encode_wide().chain(once(0)).collect()
}

 pub unsafe extern "system" fn winproc(h_wnd :HWND, msg :UINT, w_param :WPARAM, l_param :LPARAM) -> LRESULT {
        match msg {
            WM_SIZE => { println!("WM_SIZE"); },
            WM_CLOSE => {  GLOBAL_RUNNING = false; /* return user32::DefWindowProcW(h_wnd, msg, w_param, l_param); */  },
            WM_ACTIVATEAPP => { println!("WM_ACTIVATEAPP"); },
            WM_PAINT => { 
                println!("WM_PAINT"); 
                return user32::DefWindowProcW(h_wnd, msg, w_param, l_param)
            },
            _ => { return user32::DefWindowProcW(h_wnd, msg, w_param, l_param); }
        };
        0
    }