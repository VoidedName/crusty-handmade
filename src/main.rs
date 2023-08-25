

mod audio;
mod ring_buffer;
mod x_input;

mod platform;
mod crusty_handmade;

/*
TODO(voided): This is not a final platform layer!!!
- save game locations
- getting a handle to our own executable file
- asset loading path
- threading
- raw input (support for multiple keyboards)
- sleep/timeBeginPeriod
- ClipCursor
- Fullscreen
- setcursor
- QueryCancelAutoPlay
- wm actiave app
- blit speed improvement
- hardware acceleration
- get keyboard layout (international wasd)
 */

///Declares a static mut! Allows to search for specifically global muts
macro_rules! global_mut {
    ($variable:ident : $t:ty = $e:expr) => {
        pub static mut $variable: $t = $e;
    };
}

use crate::crusty_handmade::{game_update_and_render, GameOffscreenBuffer};
use global_mut;
use platform::platform_main;


fn main() {
    platform_main();
}
