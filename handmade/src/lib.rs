pub use platform_interop::*;
pub mod platform_interop;

use std::f32;

static mut SINE_ANGLE: f32 = 0.0;
static TONE_HZ: u32 = 250;
static VOLUME: f32 = 1000.0;

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

unsafe fn fill_sound_buffer(sound_buffer: &GameSoundBuffer) -> () {
    let mut write_sample: *mut i16 = sound_buffer.memory as *mut i16;
    for _ in 0..sound_buffer.sample_count {
        let sine_value = SINE_ANGLE.sin() ;
        let sample_val = (sine_value * VOLUME) as i16;
        //Channel 1
        *write_sample = sample_val;
        write_sample = write_sample.offset(1);

        //Channel 2
        *write_sample = sample_val;
        write_sample = write_sample.offset(1);

        let sound_wave_period_in_samples = sound_buffer.sample_frequency / TONE_HZ;
        SINE_ANGLE = (SINE_ANGLE + (2.0*f32::consts::PI) / sound_wave_period_in_samples as f32) % (2.0*f32::consts::PI)
    }
}

#[no_mangle]
pub fn game_update_and_render(offscreen_buffer: &mut GameOffscreenBuffer, x_offset: i16, y_offset: i16, sound_buffer: &GameSoundBuffer) -> () {
    unsafe {
        fill_sound_buffer(sound_buffer);
        draw_weird_gradient(offscreen_buffer, x_offset, y_offset);
    }
}