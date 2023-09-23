use utility::audio::{SineAudioSource, AudioSource};
use std::mem;

use types::GameOffscreenBuffer;
use types::GameSoundBuffer;

use self::types::GameInput;
use self::types::GameMemory;
use self::types::GameState;

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

unsafe fn game_output_sound(buffer: &mut GameSoundBuffer, sound: &mut SineAudioSource) {
    let rate = buffer.samples_rate;
    for chunk in buffer.buffer.chunks_mut(2) {
        let (sl, sr) = sound.sample(rate);
        chunk[0] = sl;
        chunk[1] = sr;
    }
}

/// TODO(voided): Services that the platform layer provides to the game

/// TODO(voided): Services that the game provides to the platform layer
/// It needs to take the timing, controller/keyboard input, bitmap buffer to use, sound buffer to use

///  # Safety
///  does pointer stuff, consider refactoring to be Safety
#[no_mangle]
pub unsafe fn game_update_and_render<'a>(
    game_memory: &mut GameMemory,
    inputs: &'a GameInput,
    buffer: &'a mut GameOffscreenBuffer,
    sound_buffer: &'a mut GameSoundBuffer,
) {
    debug_assert!(mem::size_of::<GameState>() <= game_memory.permanent_storage_size);
    
    // TODO(voided): currently, we can't load from the platform layer, as it is circular
    // resolve this by moving those things into their own crate
    
    let game_state = &mut *game_memory.permanent_storage.cast::<GameState>();
    if !game_memory.is_initalized {
        game_state.tone.reset_with(255, 0.3);

        game_memory.is_initalized = true;
    }

    for input in inputs {
        // TODO(voided): Allow samples offset here for more robust platform options
        match input.is_analog {
            true => {
                game_state.tone.hz = (256.0 + 128.0 * input.stick_left.y_average) as u32;
                game_state.x_offset += (4.0 * input.stick_left.x_average) as i32;
            }
            false => {
                if input.move_right.button_is_down {
                    game_state.x_offset += 1;
                }
                if input.move_left.button_is_down {
                    game_state.x_offset -= 1;
                }
            }
        }

        if input.action_down.button_is_down {
            game_state.y_offset += 1;
        }
    }
    game_output_sound(sound_buffer, &mut game_state.tone);
    render_weird_gradient(buffer, game_state.x_offset, game_state.y_offset);
}
