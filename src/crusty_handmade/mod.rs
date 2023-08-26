use crate::audio::{AudioSource, BufferAudioSource};
use crate::global_mut;
use crate::{audio::SineAudioSource, ring_buffer::RingBuffer};
use itertools::Itertools;
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

pub struct GameSoundBuffer<'a> {
    pub samples_rate: u32,
    pub buffer: &'a mut [f32],
}

unsafe fn render_weird_gradient(buffer: &mut GameOffscreenBuffer, x_offset: i32, y_offset: i32) {
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

global_mut!(OUT: SineAudioSource = SineAudioSource::new(255, 0.0));
global_mut!(X_OFFSET: i32 = 0);
global_mut!(Y_OFFSET: i32 = 0);

unsafe fn game_output_sounde(buffer: &mut GameSoundBuffer) {
    let rate = buffer.samples_rate;
    OUT.volume = 1.0;

    for mut chunk in buffer.buffer.chunks_mut(2) {
        let (sl, sr) = OUT.sample(rate);
        chunk[0] = sl;
        chunk[1] = sr;
    }
}

/// TODO(voided): Services that the platform layer provides to the game

/// TODO(voided): Services that the game provides to the platform layer
/// It needs to take the timing, controller/keyboard input, bitmap buffer to use, sound buffer to use
pub unsafe fn game_update_and_render<'a>(
    buffer: &'a mut GameOffscreenBuffer,
    x_offset: i32,
    y_offset: i32,
    sound_buffer: &'a mut GameSoundBuffer,
    hz: u32,
) {
    // TODO(voided): Allow samples offset here for more robust platform options
    OUT.hz = hz;
    X_OFFSET = x_offset;
    Y_OFFSET = y_offset;
    game_output_sounde(sound_buffer);
    render_weird_gradient(buffer, X_OFFSET, Y_OFFSET);
}
