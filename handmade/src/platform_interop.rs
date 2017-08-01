use std::os::raw::c_void;

pub type GameUpdateAndRender = extern fn(&mut GameOffscreenBuffer, i16, i16) -> ();

pub struct GameOffscreenBuffer {
    pub bytes_per_pixel: i32,
    pub width: i32,
    pub height: i32,
    pub memory: *mut c_void
}
