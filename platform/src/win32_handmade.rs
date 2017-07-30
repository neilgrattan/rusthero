extern crate user32;
extern crate winapi;
extern crate kernel32;
extern crate libc;
extern crate gdi32;

use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::mem;
use std::ptr::null_mut;
use std::f32;
use std::u32;

use self::winapi::winuser::*;
use self::winapi::windef::*;
use self::winapi::minwindef::*;
use self::winapi::wingdi::*;
use self::winapi::winnt::*;
use self::winapi::xinput::*;
use self::winapi::dsound::*;
use self::winapi::unknwnbase::LPUNKNOWN;
use self::winapi::guiddef::LPCGUID;
use self::winapi::winerror::{HRESULT, SUCCEEDED};
use self::winapi::mmreg::{WAVEFORMATEX,WAVE_FORMAT_PCM};
use self::winapi::guiddef::GUID;

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

type DirectSoundCreate = extern "system" fn(pcGuidDevice: LPCGUID, ppds: *mut LPDIRECTSOUND, pUnkOuter: LPUNKNOWN) -> HRESULT;

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

struct Win32SoundOutput {
    playing_audio: bool,
    wave_sample_index: u32,
    sine_angle: f32,
    sound_wave_hz: u32,
    sound_wave_volume: i32,
    sound_buffer: LPDIRECTSOUNDBUFFER,
    sound_buffer_bytes_per_sample: u32,
    sound_buffer_sample_frequency: u32,
    latency_sample_count: u32
}

impl Win32SoundOutput {
    fn sound_wave_period_in_samples(&self) -> u32 {
        self.sound_buffer_sample_frequency / self.sound_wave_hz
    }

    fn buffer_bytes_count(&self) -> u32 {
        self.sound_buffer_sample_frequency * self.sound_buffer_bytes_per_sample
    }
}

unsafe fn win32_load_direct_sound(window_handle: HWND, sound_output: &mut Win32SoundOutput) {
    let dsound_module = kernel32::LoadLibraryW(wstr!("dsound.dll"));

    if dsound_module != 0 as HMODULE {
        let direct_sound_create_ptr: DirectSoundCreate = mem::transmute::<FARPROC, DirectSoundCreate>(kernel32::GetProcAddress(dsound_module, cstr!("DirectSoundCreate")));
        let mut direct_sound: LPDIRECTSOUND = mem::uninitialized();
        if SUCCEEDED(direct_sound_create_ptr(0 as LPCGUID, &mut direct_sound, 0 as LPUNKNOWN)) {
            if SUCCEEDED((*direct_sound).SetCooperativeLevel(window_handle, DSSCL_PRIORITY)) != true {
                println!("Failed to set cooperative level.");
            }

            let buffer_description = DSBUFFERDESC {
                dwSize: mem::size_of::<DSBUFFERDESC>() as u32,
                dwFlags: DSBCAPS_PRIMARYBUFFER,
                dwBufferBytes: 0,
                dwReserved: 0,
                lpwfxFormat: null_mut(),
                guid3DAlgorithm: GUID {Data1: 0, Data2: 0, Data3: 0, Data4: [0;8] }
            };

            let mut primary_buffer: LPDIRECTSOUNDBUFFER = mem::uninitialized();
            if !SUCCEEDED((*direct_sound).CreateSoundBuffer(&buffer_description, &mut primary_buffer, 0 as LPUNKNOWN)) {
                println!("Failed to create primary buffer");
            }

            let mut wave_format: WAVEFORMATEX = mem::zeroed();
            wave_format.wFormatTag      = WAVE_FORMAT_PCM;
            wave_format.nChannels       = 2;
            wave_format.nSamplesPerSec  = sound_output.sound_buffer_sample_frequency;
            wave_format.wBitsPerSample  = 16;
            wave_format.nBlockAlign     = (wave_format.nChannels * wave_format.wBitsPerSample) / 8;
            wave_format.nAvgBytesPerSec = wave_format.nSamplesPerSec * wave_format.nBlockAlign as DWORD;
            wave_format.cbSize          = 0;

            if !SUCCEEDED((*primary_buffer).SetFormat(&wave_format)) {
                println!("Failed to set format of primary buffer");
            }

            let buffer_description = DSBUFFERDESC {
                dwSize: mem::size_of::<DSBUFFERDESC>() as u32,
                dwFlags: DSBCAPS_GETCURRENTPOSITION2,
                dwBufferBytes: sound_output.buffer_bytes_count(),
                dwReserved: 0,
                lpwfxFormat: &mut wave_format,
                guid3DAlgorithm: GUID {Data1: 0, Data2: 0, Data3: 0, Data4: [0;8] }
            };

            if !SUCCEEDED((*direct_sound).CreateSoundBuffer(&buffer_description, &mut sound_output.sound_buffer, null_mut())) {
                println!("Failed to create sound buffer!");
            }
        }
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
    
    buffer.memory = kernel32::VirtualAlloc(0 as LPVOID, bitmap_memory_size as u64, MEM_RESERVE|MEM_COMMIT, PAGE_READWRITE);
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

unsafe fn win32_fill_sound_buffer(sound_output: &mut Win32SoundOutput, byte_to_lock: DWORD, num_bytes_to_write: DWORD) {
    let mut section_1_pointer = null_mut();
    let mut section_1_size = 0;
    let mut section_2_pointer = null_mut();
    let mut section_2_size = 0;

    if SUCCEEDED((*sound_output.sound_buffer).Lock(byte_to_lock, num_bytes_to_write, &mut section_1_pointer, &mut section_1_size, &mut section_2_pointer, &mut section_2_size, 0)) {

        let mut write_sample: *mut i16 = section_1_pointer as *mut i16;
        for write_index in 0..section_1_size/(sound_output.sound_buffer_bytes_per_sample as u32) {
            let sine_value = sound_output.sine_angle.sin();
            let sample_val = (sine_value * sound_output.sound_wave_volume as f32) as i16;

            //Channel 1
            *write_sample = sample_val;
            write_sample = write_sample.offset(1);

            //Channel 2
            *write_sample = sample_val;
            write_sample = write_sample.offset(1);

            sound_output.wave_sample_index = sound_output.wave_sample_index.wrapping_add(1);
            sound_output.sine_angle = sound_output.sine_angle + (2.0*f32::consts::PI) / sound_output.sound_wave_period_in_samples() as f32
        }

        let mut write_sample: *mut i16 = section_2_pointer as *mut i16;
        for write_index in 0..section_2_size/(sound_output.sound_buffer_bytes_per_sample as u32) {
            let sine_value = sound_output.sine_angle.sin();
            let sample_val = (sine_value * sound_output.sound_wave_volume as f32) as i16;

            //Channel 1
            *write_sample = sample_val;
            write_sample = write_sample.offset(1);

            //Channel 2
            *write_sample = sample_val;
            write_sample = write_sample.offset(1);

            sound_output.wave_sample_index = sound_output.wave_sample_index.wrapping_add(1);
            sound_output.sine_angle = sound_output.sine_angle + (2.0*f32::consts::PI) / sound_output.sound_wave_period_in_samples() as f32
        }
        
        if !SUCCEEDED((*sound_output.sound_buffer).Unlock(section_1_pointer, section_1_size, section_2_pointer, section_2_size)) {
            println!("Failed to unlock");
        }
    }
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
        //let pan_speed = 10;

        let mut sound_output = Win32SoundOutput {
            playing_audio: false,
            wave_sample_index: 0, //wave position in samples
            sine_angle: 0.0,
            sound_wave_hz: 256,
            sound_wave_volume: 1000,
            sound_buffer: 0 as LPDIRECTSOUNDBUFFER,
            sound_buffer_bytes_per_sample: 4,
            sound_buffer_sample_frequency: 48000,
            latency_sample_count: 48000 / 15  // Number of samples to stay head of the play cursor
        };

        win32_load_direct_sound(window_handle, &mut sound_output);
        let buffer_fill_bytes = sound_output.latency_sample_count * sound_output.sound_buffer_bytes_per_sample;
        win32_fill_sound_buffer(&mut sound_output, 0, buffer_fill_bytes);

        let mut perf_counter_frequency = 0;
        kernel32::QueryPerformanceFrequency(&mut perf_counter_frequency);
        
        let mut last_counter = 0;
        kernel32::QueryPerformanceCounter(&mut last_counter);

        while GLOBAL_RUNNING {
            let mut msg: MSG = mem::uninitialized();
            while user32::PeekMessageW(&mut msg, window_handle,0,0,PM_REMOVE) != 0 {
                user32::TranslateMessage(&mut msg);
                user32::DispatchMessageW(&mut msg);
            }

            let mut split1 = 0;
            kernel32::QueryPerformanceCounter(&mut split1);

            for controller_index in 0..XUSER_MAX_COUNT {
                let mut controller_state = mem::uninitialized();          
                let controller_found = XINPUT_GET_STATE_PTR(controller_index as DWORD, &mut controller_state);
                if controller_found == self::winapi::winerror::ERROR_SUCCESS {
                    let pad = controller_state.Gamepad;
                    
                    // let dpad_up = pad.wButtons & XINPUT_GAMEPAD_DPAD_UP;
                    // let dpad_down = pad.wButtons & XINPUT_GAMEPAD_DPAD_DOWN;
                    // let dpad_left = pad.wButtons & XINPUT_GAMEPAD_DPAD_LEFT;
                    // let dpad_right = pad.wButtons & XINPUT_GAMEPAD_DPAD_RIGHT;
                    // let a = pad.wButtons & XINPUT_GAMEPAD_A;
                    // let b = pad.wButtons & XINPUT_GAMEPAD_B;
                    // let x = pad.wButtons & XINPUT_GAMEPAD_X;
                    // let y = pad.wButtons & XINPUT_GAMEPAD_Y;
                    // let start = pad.wButtons & XINPUT_GAMEPAD_START;
                    // let back = pad.wButtons & XINPUT_GAMEPAD_BACK;
                    // let left_shoulder = pad.wButtons & XINPUT_GAMEPAD_LEFT_SHOULDER;
                    // let right_shoulder = pad.wButtons & XINPUT_GAMEPAD_RIGHT_SHOULDER;

                    let stick_x = pad.sThumbLX;
                    let stick_y = pad.sThumbLY;

                    if stick_x.abs() > XINPUT_GAMEPAD_LEFT_THUMB_DEADZONE { x_offset += stick_x >> 12;  sound_output.sound_wave_hz = (sound_output.sound_wave_hz as i32 + ((stick_x >> 12)as i32 / 2)) as u32 ; }
                    if stick_y.abs() > XINPUT_GAMEPAD_LEFT_THUMB_DEADZONE { y_offset += stick_y >> 12; }

                    //XINPUT_SET_STATE_PTR(controller_index, &mut XINPUT_VIBRATION { wLeftMotorSpeed: 60000, wRightMotorSpeed: 60000 });
                } else {
                    //TODO: Check if we care about this controller.
                }
            }

            let mut split2 = 0;
            kernel32::QueryPerformanceCounter(&mut split2);

            // Test graphics
            draw_weird_gradient(&mut OFFSCREEN_BUFFER, x_offset, y_offset);

            let mut split3 = 0;
            kernel32::QueryPerformanceCounter(&mut split3);
            
            let window_dimensions = win32_get_window_dimensions(window_handle);
            win32_update_window(device_context, &mut OFFSCREEN_BUFFER, window_dimensions.width, window_dimensions.height);

            

            // Test sound
            let mut play_cursor = 0;
            let mut write_cursor = 0;

            if SUCCEEDED((*sound_output.sound_buffer).GetCurrentPosition(&mut play_cursor, &mut write_cursor)) {
                let byte_to_lock = (sound_output.wave_sample_index.wrapping_mul(sound_output.sound_buffer_bytes_per_sample as u32)) % sound_output.buffer_bytes_count();
                let target_cursor = (play_cursor + (sound_output.latency_sample_count*sound_output.sound_buffer_bytes_per_sample)) % sound_output.buffer_bytes_count();

                let num_bytes_to_write = if byte_to_lock > target_cursor {
                    (sound_output.buffer_bytes_count() - byte_to_lock) + target_cursor
                } 
                else {
                    target_cursor - byte_to_lock
                };

                win32_fill_sound_buffer(&mut sound_output, byte_to_lock, num_bytes_to_write);

                if !sound_output.playing_audio {
                    (*sound_output.sound_buffer).Play(0, 0, DSBPLAY_LOOPING);
                    sound_output.playing_audio = true;
                }
            }

            let mut end_counter = 0;
            kernel32::QueryPerformanceCounter(&mut end_counter);

            let split1_elapsed = 1000*(split1 - last_counter) / (perf_counter_frequency);  //Devide by frequency to get how many cycles per second
            let split2_elapsed = 1000*(split2 - split1) / (perf_counter_frequency);  //Devide by frequency to get how many cycles per second
            let split3_elapsed = 1000*(split3 - split2) / (perf_counter_frequency);  //Devide by frequency to get how many cycles per second
            let total_elapsed = 1000.0 *(end_counter - last_counter) as f32 / (perf_counter_frequency as f32);  //Devide by frequency to get how many cycles per second
            let fps = 1000.0 / total_elapsed;
            println!("Split1 {}, Split2 {}, Split3 {}, Total {:.2}ms, FPS {:.2}", split1_elapsed, split2_elapsed, split3_elapsed, total_elapsed, fps);
            last_counter = end_counter;
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