#[no_mangle]
pub fn game_update_and_render(myfun: extern fn(i32) -> ()) -> () {
    //println!("OIOIOIOIA!!! clib {:?}", somes.my_cool_guid);
    myfun(3);
}