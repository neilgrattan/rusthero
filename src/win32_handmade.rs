extern crate user32;
extern crate winapi;
extern crate kernel32;
extern crate libc;
extern crate gdi32;
use std::ffi::OsStr;
// use std::io::Error;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
// use std::ptr::null_mut;
use std::mem;
use self::winapi::winuser::{WNDCLASSW,CS_HREDRAW,CS_VREDRAW,WS_OVERLAPPEDWINDOW,CW_USEDEFAULT,MSG,WS_VISIBLE,PM_REMOVE,WM_PAINT,WM_SIZE,WM_CLOSE,WM_ACTIVATEAPP,PAINTSTRUCT};
use self::winapi::windef::{HWND,HBRUSH,HMENU};
use self::winapi::minwindef::{UINT,WPARAM,LPARAM,LRESULT,LPVOID};
use self::winapi::wingdi::{WHITENESS,BLACKNESS, PATINVERT};

static mut GLOBAL_RUNNING: bool = true; //TODO: Whats the correct way to scope something like this?

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

    let ret = unsafe { //TODO: Can we use Options to get rid of all these unsafes?
        user32::RegisterClassW(&mut window)
    };

    if ret == 0 {
        println!("Registering the window class failed.");
        return
    }
    
    unsafe { 
         let window_handle = user32::CreateWindowExW(
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
        );

        if window_handle.is_null() {
            println!("Failed to create window.");
            return;
        }

        while GLOBAL_RUNNING {
            let mut msg: MSG = mem::uninitialized();
            while user32::PeekMessageW(&mut msg, window_handle,0,0,PM_REMOVE) != 0 {
                user32::TranslateMessage(&mut msg);
                user32::DispatchMessageW(&mut msg);
            }
        }
    }
}

fn to_wide_string(str: &str) -> Vec<u16> {
    OsStr::new(str).encode_wide().chain(once(0)).collect()
}

 pub unsafe extern "system" fn winproc(hwnd :HWND, msg :UINT, w_param :WPARAM, l_param :LPARAM) -> LRESULT {
        match msg {
            WM_SIZE => { println!("WM_SIZE"); },
            WM_CLOSE => {  GLOBAL_RUNNING = false; },
            WM_ACTIVATEAPP => { println!("WM_ACTIVATEAPP"); },
            WM_PAINT => {
                let mut paint: PAINTSTRUCT = mem::uninitialized();
                let hdc = user32::BeginPaint(hwnd, &mut paint);
                let x = paint.rcPaint.left;
                let y = paint.rcPaint.top;
                let width = paint.rcPaint.right - paint.rcPaint.left;
                let height = paint.rcPaint.bottom - paint.rcPaint.top;
                gdi32::PatBlt(hdc, x, y, width, height, PATINVERT);
            },
            _ => { return user32::DefWindowProcW(hwnd, msg, w_param, l_param); }
        };
        0
    }