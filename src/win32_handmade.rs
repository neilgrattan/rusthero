extern crate user32;
extern crate winapi;
extern crate kernel32;
extern crate libc;
extern crate gdi32;

use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::mem;
use std::num::Wrapping;

use self::winapi::winuser::*;
use self::winapi::windef::*;
use self::winapi::minwindef::*;
use self::winapi::wingdi::*;
use self::winapi::winnt::*;
use self::winapi::xinput::*;
// Dynamic Linking XInput because reasons.  See the initialising code
type XInputGetStateType = extern "system" fn(user_index:DWORD, state: *mut XINPUT_STATE) -> DWORD;
extern "system" fn xinput_get_state_stub(_: DWORD, _: *mut XINPUT_STATE) -> DWORD {
    self::winapi::winerror::ERROR_DEVICE_NOT_CONNECTED
}
static mut XINPUT_GET_STATE_PTR: XInputGetStateType = xinput_get_state_stub;

type XInputSetStateType = extern "system" fn(user_index:DWORD, state: *mut XINPUT_VIBRATION) -> DWORD;
extern "system" fn xinput_set_state_stub(_: DWORD, _: *mut XINPUT_VIBRATION) -> DWORD {
    self::winapi::winerror::ERROR_DEVICE_NOT_CONNECTED
}
static mut XINPUT_SET_STATE_PTR: XInputSetStateType = xinput_set_state_stub;


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

macro_rules! cstr {
    ($str:expr) => ({
        use std::ffi::CString;
        CString::new($str).unwrap().as_ptr()
    });
}

macro_rules! wstr {
    ($str:expr) => ({
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        let wstr: Vec<u16> = OsStr::new($str)
                                 .encode_wide()
                                 .chain(Some(0).into_iter())
                                 .collect();
        wstr.as_ptr()
    });
}

unsafe fn win32_load_xinput() {
        //Version available on modern windows without DX SDK Install
        let mut xinput_module = kernel32::LoadLibraryW(wstr!("xinput1_4.dll"));

        if xinput_module == 0 as HMODULE {
            //Version available on old windows without DX SDK Install
            xinput_module = kernel32::LoadLibraryW(wstr!("xinput9_1_0.dll"));
        }
        
        if xinput_module != 0 as HMODULE {
            XINPUT_GET_STATE_PTR = mem::transmute::<FARPROC, XInputGetStateType>(kernel32::GetProcAddress(xinput_module, cstr!("XInputGetState")));
            XINPUT_SET_STATE_PTR = mem::transmute::<FARPROC, XInputSetStateType>(kernel32::GetProcAddress(xinput_module, cstr!("XInputSetState")));
        }
}

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

fn to_wide_string(str: &str) -> Vec<u16> {
    OsStr::new(str).encode_wide().chain(once(0)).collect()
}

unsafe fn draw_weird_gradient(buffer: &mut Win32OffscreenBuffer, x_offset: i16, y_offset: i16) {
    
    let width = buffer.width as i16;
    let height = buffer.height as i16;
    let bytes_per_pixel = buffer.bytes_per_pixel as i16;
    
    let pitch = width*bytes_per_pixel;
    let mut row: *mut u8 = buffer.memory as *mut u8;
    for y in 0..height {
        let mut pixel: *mut u32 = row as *mut u32;
        for x in 0..width {
            *pixel = ((x.wrapping_add(x_offset) as u8) << 0) as u32 + (((y.wrapping_add(-y_offset) as u8) as u32) << 8);
            pixel = pixel.offset(1);

        };
        row = row.offset(pitch as isize);
    };
}

unsafe fn win32_update_window(device_context: HDC, buffer: &mut Win32OffscreenBuffer, window_width: i32, window_height: i32) {
    gdi32::StretchDIBits(device_context, 
        0, 0, window_width, window_height,
        0, 0, buffer.width, buffer.height,
        buffer.memory, 
        &buffer.info, 
        DIB_RGB_COLORS, 
        SRCCOPY);
}

pub fn main() {
    unsafe {
        OFFSCREEN_BUFFER.info.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
        win32_resize_dib_section(&mut OFFSCREEN_BUFFER, 1280, 720);
        win32_load_xinput();
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

        let mut x_offset = 0;
        let mut y_offset = 0;
        let pan_speed = 10;

        while GLOBAL_RUNNING {
            let mut msg: MSG = mem::uninitialized();
            while user32::PeekMessageW(&mut msg, window_handle,0,0,PM_REMOVE) != 0 {
                user32::TranslateMessage(&mut msg);
                user32::DispatchMessageW(&mut msg);
            }

            for controller_index in 0..XUSER_MAX_COUNT {
                let mut controller_state = mem::uninitialized();          
                let controller_found = XINPUT_GET_STATE_PTR(controller_index as DWORD, &mut controller_state);
                if controller_found == self::winapi::winerror::ERROR_SUCCESS {
                    let pad = controller_state.Gamepad;
                    
                    let dpad_up = pad.wButtons & XINPUT_GAMEPAD_DPAD_UP;
                    let dpad_down = pad.wButtons & XINPUT_GAMEPAD_DPAD_DOWN;
                    let dpad_left = pad.wButtons & XINPUT_GAMEPAD_DPAD_LEFT;
                    let dpad_right = pad.wButtons & XINPUT_GAMEPAD_DPAD_RIGHT;
                    let a = pad.wButtons & XINPUT_GAMEPAD_A;
                    let b = pad.wButtons & XINPUT_GAMEPAD_B;
                    let x = pad.wButtons & XINPUT_GAMEPAD_X;
                    let y = pad.wButtons & XINPUT_GAMEPAD_Y;
                    let start = pad.wButtons & XINPUT_GAMEPAD_START;
                    let back = pad.wButtons & XINPUT_GAMEPAD_BACK;
                    let left_shoulder = pad.wButtons & XINPUT_GAMEPAD_LEFT_SHOULDER;
                    let right_shoulder = pad.wButtons & XINPUT_GAMEPAD_RIGHT_SHOULDER;

                    let stick_x = pad.sThumbLX;
                    let stick_y = pad.sThumbLY;

                    if stick_x.abs() > XINPUT_GAMEPAD_LEFT_THUMB_DEADZONE { x_offset += stick_x >> 12; }
                    if stick_y.abs() > XINPUT_GAMEPAD_LEFT_THUMB_DEADZONE { y_offset += stick_y >> 12; }

                    //XINPUT_SET_STATE_PTR(controller_index, &mut XINPUT_VIBRATION { wLeftMotorSpeed: 60000, wRightMotorSpeed: 60000 });
                } else {
                    //TODO: Check if we care about this controller.
                }
            }



            // x += 1;
            // y += 1;
            draw_weird_gradient(&mut OFFSCREEN_BUFFER, x_offset, y_offset);
            
            let window_dimensions = win32_get_window_dimensions(window_handle);
            win32_update_window(device_context, &mut OFFSCREEN_BUFFER, window_dimensions.width, window_dimensions.height);
        }
    }
}

 pub unsafe extern "system" fn winproc(hwnd :HWND, msg :UINT, w_param :WPARAM, l_param :LPARAM) -> LRESULT {
    match msg {
        WM_SIZE => {  },
        WM_CLOSE => {  GLOBAL_RUNNING = false;  },
        WM_DESTROY => { GLOBAL_RUNNING = false; },
        WM_ACTIVATEAPP => { println!("WM_ACTIVATEAPP"); },
        WM_PAINT => {
            let mut paint: PAINTSTRUCT = mem::uninitialized();
            let hdc = user32::BeginPaint(hwnd, &mut paint);
            let window_dimensions = win32_get_window_dimensions(hwnd);
            win32_update_window(hdc, &mut OFFSCREEN_BUFFER, window_dimensions.width, window_dimensions.height);
            user32::EndPaint(hwnd, &paint);
        },
        WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP => {
            let vk_code = w_param;
            let was_down = (l_param & 1 << 30) != 0;
            let is_down = (l_param & 1 << 31) == 0;
            let alt_down = (l_param & 1 << 29) != 0;

            if vk_code == 'W' as u64 {
            }
            if vk_code == 'A' as u64 {
            }
            if vk_code == 'S' as u64 {
            }
            if vk_code == 'D' as u64 {
            }
            if vk_code == 'Q' as u64 {
            }
            if vk_code == VK_UP as u64 {
            }
            if vk_code == VK_DOWN as u64 {
            }
            if vk_code == VK_LEFT as u64 {
            }
            if vk_code == VK_RIGHT as u64 {
            }
            if vk_code == VK_ESCAPE as u64 {
                if is_down != was_down {
                    println!("ESCAPE");
                }
            }
            if vk_code == VK_SPACE as u64 {
                println!("\n");
            }
            if vk_code == VK_F4 as u64 && alt_down {
                GLOBAL_RUNNING = false;
             }

        },
        _ => { return user32::DefWindowProcW(hwnd, msg, w_param, l_param); }
    };
    0
}