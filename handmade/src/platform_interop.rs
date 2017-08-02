use std::os::raw::c_void;

pub type GameUpdateAndRender = extern fn(&mut GameOffscreenBuffer, i16, i16, &GameSoundBuffer) -> ();

pub struct GameOffscreenBuffer {
    pub bytes_per_pixel: i32,
    pub width: i32,
    pub height: i32,
    pub memory: *mut c_void
}

pub struct GameSoundBuffer {
    pub memory: *mut c_void,
    pub sample_count: u32,
    pub sample_frequency: u32
}