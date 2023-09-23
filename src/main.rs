mod platform;

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

#[hot_lib_reloader::hot_module(dylib = "crusty_handmade")]
mod hot_lib {
    hot_functions_from_file!("crusty_handmade/src/lib.rs");
    pub use crusty_handmade::types::*;
}

/// TODO(voided) make this conditional 
mod game {
    use crate::hot_lib;
    pub use hot_lib::*;
}

use global_mut;
use platform::platform_main;

fn main() {
    platform_main();
}
