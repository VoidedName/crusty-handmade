use crate::audio::AudioSource;
use crate::audio::SineAudioSource;
use crate::crusty_handmade::types::GameOffscreenBuffer;
use crate::crusty_handmade::types::GameSoundBuffer;
use crate::global_mut;

use self::types::GameInput;

pub mod types;

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

    for chunk in buffer.buffer.chunks_mut(2) {
        let (sl, sr) = OUT.sample(rate);
        chunk[0] = sl;
        chunk[1] = sr;
    }
}

/// TODO(voided): Services that the platform layer provides to the game

/// TODO(voided): Services that the game provides to the platform layer
/// It needs to take the timing, controller/keyboard input, bitmap buffer to use, sound buffer to use
pub unsafe fn game_update_and_render<'a>(
    inputs: &'a GameInput,
    buffer: &'a mut GameOffscreenBuffer,
    sound_buffer: &'a mut GameSoundBuffer,
) {
    // TODO(voided): Allow samples offset here for more robust platform options
    let input0 = &inputs[0];

    match input0.is_analog {
        true => {
            OUT.hz = (256.0 + 128.0 * input0.stick_left.y_axis.end) as u32;
            X_OFFSET += (4.0 * input0.stick_left.x_axis.end) as i32;
        }
        false => {}
    }

    if input0.button_down.button_is_down {
        Y_OFFSET += 1;
    }

    OUT.volume = 0.3;
    game_output_sounde(sound_buffer);
    render_weird_gradient(buffer, X_OFFSET, Y_OFFSET);
}
