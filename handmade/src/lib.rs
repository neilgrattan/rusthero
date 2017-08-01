pub use platform_interop::*;
pub mod platform_interop;

#[no_mangle]
pub fn game_update_and_render(offscreen_buffer: &mut GameOffscreenBuffer, x_offset: i16, y_offset: i16) -> () {
    unsafe {draw_weird_gradient(offscreen_buffer, x_offset, y_offset);}
}

unsafe fn draw_weird_gradient(buffer: &mut GameOffscreenBuffer, x_offset: i16, y_offset: i16) {
    
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