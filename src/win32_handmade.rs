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

struct Win32OffscreenBuffer {
    info: BITMAPINFO,
    bytes_per_pixel: i32,
    width: i32,
    height: i32,
    memory: LPVOID
}

struct WindowDimensions {
    width: i32,
    height: i32
}

static mut GLOBAL_RUNNING: bool = true;
static mut REFRESH_WINDOW_DIMENSIONS: bool = false;

static mut OFFSCREEN_BUFFER: Win32OffscreenBuffer = Win32OffscreenBuffer {
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
    },
    bytes_per_pixel: 4,
    width: 0,
    height: 0,
    memory: 0 as LPVOID
};

unsafe fn win32_get_window_dimensions (window_handle: HWND) -> WindowDimensions {
    let mut window_rect: RECT = mem::uninitialized();
    user32::GetClientRect(window_handle, &mut window_rect);
    WindowDimensions {
        width: window_rect.right - window_rect.left,
        height: window_rect.bottom - window_rect.top
    }
}

unsafe fn win32_resize_dib_section(buffer: &mut Win32OffscreenBuffer, width: i32, height: i32) -> () {
    buffer.width = width;
    buffer.height = height;
    buffer.info.bmiHeader.biWidth = width;
    buffer.info.bmiHeader.biHeight = -height; //Top-down coord

    let bitmap_memory_size = (width*height)*buffer.bytes_per_pixel;

    if buffer.memory != 0 as LPVOID {
        kernel32::VirtualFree(buffer.memory, 0 as u64, MEM_RELEASE);
    }
    
    buffer.memory = kernel32::VirtualAlloc(0 as LPVOID, bitmap_memory_size as u64, MEM_COMMIT, PAGE_READWRITE);

}

unsafe fn draw_weird_gradient(buffer: &mut Win32OffscreenBuffer, x_offset: u32, y_offset: u32) {
    
    let width = buffer.width;
    let height = buffer.height;
    let bytes_per_pixel = buffer.bytes_per_pixel;
    
    let pitch = width*bytes_per_pixel;
    let mut row: *mut u8 = buffer.memory as *mut u8;
    for y in 0..height {
        let mut pixel: *mut u32 = row as *mut u32;
        for x in 0..width {
            *pixel = ((x as u32 + x_offset) << 0 ) + ((y as u32 + y_offset) << 8);
            pixel = pixel.offset(1);

        };
        row = row.offset(pitch as isize);
    };
}

unsafe fn win32_update_window(device_context: HDC, buffer: &mut Win32OffscreenBuffer, window_width: i32, window_height: i32) {
    gdi32::StretchDIBits(device_context, 
        0, 0, buffer.width, buffer.height,
        0, 0, window_width, window_height,
        buffer.memory, 
        &buffer.info, 
        DIB_RGB_COLORS, 
        SRCCOPY);
}

pub fn main() {
    unsafe {
        OFFSCREEN_BUFFER.info.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
        OFFSCREEN_BUFFER.info.bmiHeader.biWidth = 1080;
        OFFSCREEN_BUFFER.info.bmiHeader.biHeight = 640;
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
        let mut window_dimensions = win32_get_window_dimensions(window_handle);

        let mut x = 0;
        let mut y = 0;
        while GLOBAL_RUNNING {
            let mut msg: MSG = mem::uninitialized();
            while user32::PeekMessageW(&mut msg, window_handle,0,0,PM_REMOVE) != 0 {
                user32::TranslateMessage(&mut msg);
                user32::DispatchMessageW(&mut msg);
            }
            
            x += 1;
            y += 1;
            draw_weird_gradient(&mut OFFSCREEN_BUFFER, x,y);
            
            if REFRESH_WINDOW_DIMENSIONS {
                window_dimensions = win32_get_window_dimensions(window_handle);
                REFRESH_WINDOW_DIMENSIONS = false;
            }
            win32_update_window(device_context, &mut OFFSCREEN_BUFFER, window_dimensions.width, window_dimensions.height);
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
            let window_dimensions = win32_get_window_dimensions(hwnd);
            win32_resize_dib_section(&mut OFFSCREEN_BUFFER, window_dimensions.width, window_dimensions.height);
            REFRESH_WINDOW_DIMENSIONS = true;
        },
        WM_CLOSE => {  GLOBAL_RUNNING = false;  },
        WM_DESTROY => { GLOBAL_RUNNING = false; },
        WM_ACTIVATEAPP => { println!("WM_ACTIVATEAPP"); },
        WM_PAINT => {
            println!("WM_PAINT");
            let mut paint: PAINTSTRUCT = mem::uninitialized();
            let hdc = user32::BeginPaint(hwnd, &mut paint);
            let window_dimensions = win32_get_window_dimensions(hwnd);
            win32_update_window(hdc, &mut OFFSCREEN_BUFFER, window_dimensions.width, window_dimensions.height);
            user32::EndPaint(hwnd, &paint);
        },
        _ => { return user32::DefWindowProcW(hwnd, msg, w_param, l_param); }
    };
    0
}