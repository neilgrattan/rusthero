extern crate user32;
extern crate winapi;
extern crate kernel32;
extern crate libc;
extern crate gdi32;
use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::mem;

use self::winapi::winuser::*;
use self::winapi::windef::*;
use self::winapi::minwindef::*;
use self::winapi::wingdi::*;
use self::winapi::winnt::*;

struct MyBitmapType {
    info: BITMAPINFO
}

static mut GLOBAL_RUNNING: bool = true;
static mut WIN32_BITMAP_INFO: MyBitmapType = MyBitmapType {
    info: BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: 0,
            biWidth: 0,
            biHeight: 0,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0
        } ,
        bmiColors: []
    }
};

static mut BITMAP_MEMORY: LPVOID = 0 as LPVOID;

unsafe fn win32_resize_dib_section(width: i32, height: i32) -> () {
    
    WIN32_BITMAP_INFO.info.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
    WIN32_BITMAP_INFO.info.bmiHeader.biWidth = width;
    WIN32_BITMAP_INFO.info.bmiHeader.biHeight = -height; //Top-down coord

    let bytes_per_pixel = 4;
    let bitmap_memory_size = (width*height)*bytes_per_pixel;

    if BITMAP_MEMORY != 0 as LPVOID {
        kernel32::VirtualFree(BITMAP_MEMORY, 0 as u64, MEM_RELEASE);
    }
    
    BITMAP_MEMORY = kernel32::VirtualAlloc(0 as LPVOID, bitmap_memory_size as u64, MEM_COMMIT, PAGE_READWRITE);

    let pitch = width*bytes_per_pixel;
    let mut row: *mut u8 = BITMAP_MEMORY as *mut u8;
    for y in 0..height {
        let mut pixel: *mut u32 = row as *mut u32;
        for x in 0..width {
            //*pixel = 255 << 16; //red
            *pixel = ((x as u32) << 0) + ((y as u32) << 8);  //green
            // *pixel = 255 << 0; //blue
            pixel = pixel.offset(1);

        };
        row = row.offset(pitch as isize);
    };

}

unsafe fn win32_update_window(device_context: HDC, window_width: i32, window_height: i32) {
    gdi32::StretchDIBits(device_context, 
        0, 0, WIN32_BITMAP_INFO.info.bmiHeader.biWidth, WIN32_BITMAP_INFO.info.bmiHeader.biHeight.abs(), //We have a negative value for this to specify top-down bitmap
        0, 0, window_width, window_height,
        BITMAP_MEMORY, 
        &WIN32_BITMAP_INFO.info, 
        DIB_RGB_COLORS, 
        SRCCOPY);
}

pub fn main() {

    unsafe {
        WIN32_BITMAP_INFO.info.bmiHeader.biWidth = 1080;
        WIN32_BITMAP_INFO.info.bmiHeader.biHeight = 640;
    }

    let window_class_name = to_wide_string("RustHeroWindowClass").as_ptr();
    let h_instance = unsafe {
        kernel32::GetModuleHandleW(0 as winapi::winnt::LPCWSTR)
    };

    let mut window = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
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

        let device_context = user32::GetDC(window_handle);

        while GLOBAL_RUNNING {
            let mut msg: MSG = mem::uninitialized();
            while user32::PeekMessageW(&mut msg, window_handle,0,0,PM_REMOVE) != 0 {
                user32::TranslateMessage(&mut msg);
                user32::DispatchMessageW(&mut msg);
            }

            // win32_update_window(device_context, )
        }
    }
}

fn to_wide_string(str: &str) -> Vec<u16> {
    OsStr::new(str).encode_wide().chain(once(0)).collect()
}

 pub unsafe extern "system" fn winproc(hwnd :HWND, msg :UINT, w_param :WPARAM, l_param :LPARAM) -> LRESULT {
        match msg {
            WM_SIZE => { 
                println!("WM_SIZE"); 
                let mut client_rect: RECT = mem::uninitialized();
                user32::GetClientRect(hwnd, &mut client_rect);
                win32_resize_dib_section(client_rect.right - client_rect.left, client_rect.bottom - client_rect.top);
            },
            WM_CLOSE => {  GLOBAL_RUNNING = false;  },
            WM_DESTROY => { GLOBAL_RUNNING = false; },
            WM_ACTIVATEAPP => { println!("WM_ACTIVATEAPP"); },
            WM_PAINT => {
                let mut paint: PAINTSTRUCT = mem::uninitialized();
                let hdc = user32::BeginPaint(hwnd, &mut paint);
                let mut client_rect: RECT = mem::uninitialized();
                user32::GetClientRect(hwnd, &mut client_rect);
                win32_update_window(hdc, client_rect.right, client_rect.bottom);
                user32::EndPaint(hwnd, &paint);
            },
            _ => { return user32::DefWindowProcW(hwnd, msg, w_param, l_param); }
        };
        0
    }