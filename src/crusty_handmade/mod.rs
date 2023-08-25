use std::ffi::c_void;

pub struct GameOffscreenBuffer {
   pub memory: *mut c_void,
   pub width: i32,
   pub height: i32,
   pub bytes_per_pixel: i32,
}

impl GameOffscreenBuffer {
    pub fn pitch(&self) -> isize {
        (self.width * self.bytes_per_pixel) as isize
    }

    #[allow(unused)]
    pub fn memory_size(&self) -> i32 {
        self.bytes_per_pixel * self.width * self.height
    }
}

/// TODO(voided): Services that the platform layer provides to the game

/// TODO(voided): Services that the game provides to the platform layer
/// It needs to take the timing, controller/keyboard input, bitmap buffer to use, sound buffer to use
pub unsafe fn game_update_and_render(buffer: &mut GameOffscreenBuffer, x_offset: i32, y_offset: i32) {
    let mut row = buffer.memory.cast::<u8>();
    let pitch = buffer.pitch();

    for y in 0..buffer.height {
        let mut pixel = row.cast::<u32>();
        for x in 0..buffer.width {
            let blue = (x + x_offset) as u32 & 0xFF;
            let green = (y + y_offset) as u32 & 0xFF;
            let a = green << 8 | blue;
            *pixel = a;
            pixel = pixel.offset(1);
        }
        row = row.offset(pitch);
    }
}
