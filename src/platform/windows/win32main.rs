use crate::crusty_handmade::types::{
    ButtonInput, GameControllerInput, GameInput, GameMemory, GameOffscreenBuffer, GameSoundBuffer,
};
use crate::crusty_handmade::{gigabytes, megabytes};
use crate::game_update_and_render;
use crate::platform::windows::win32_x_input::{
    load_xinput, XInputGamepad, XinputState, XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B,
    XINPUT_GAMEPAD_BACK, XINPUT_GAMEPAD_DPAD_DOWN, XINPUT_GAMEPAD_DPAD_LEFT,
    XINPUT_GAMEPAD_DPAD_RIGHT, XINPUT_GAMEPAD_DPAD_UP, XINPUT_GAMEPAD_LEFT_SHOULDER,
    XINPUT_GAMEPAD_RIGHT_SHOULDER, XINPUT_GAMEPAD_START, XINPUT_GAMEPAD_X, XINPUT_GAMEPAD_Y,
    XINPUT_GET_STATE, XUSER_MAX_COUNT,
};
use crate::utility::ring_buffer::RingBuffer;
use std::arch::x86_64::_rdtsc;
use std::cmp::max;
use std::ffi::c_void;
use std::fmt::Debug;
use std::mem;
use std::ptr::null_mut;
use std::time::Duration;

use windows::Win32::Media::{timeBeginPeriod, TIMERR_NOERROR};
use windows::Win32::System::Memory::{
    VirtualAlloc, VirtualFree, MEM_COMMIT, MEM_RELEASE, PAGE_READWRITE,
};
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::{
    core::*, Win32::Foundation::*, Win32::Graphics::Gdi::*,
    Win32::System::LibraryLoader::GetModuleHandleW, Win32::UI::WindowsAndMessaging::*,
};

use crate::global_mut;
use crate::utility::audio::{AudioOutput, BufferAudioSource, ThreadSharedAudioSource};

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum RunState {
    Starting,
    Running,
    Stopping,
}

macro_rules! has_bitflag {
    ($value:expr, $flag:expr) => {
        $flag & $value == $flag
    };
}

global_mut!(PERFORMANCE_FREQUENCY: i64 = 0);
//TODO(voided): This is a global for now.
global_mut!(RUN_STATE: RunState = RunState::Starting);
global_mut!(GLOBAL_BACK_BUFFER: OffscreenBuffer = OffscreenBuffer {
    info: BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: 0,
            biWidth: 0,
            biHeight: 0,
            biPlanes: 0,
            biBitCount: 0,
            biCompression: 0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [RGBQUAD{
            rgbBlue: 0,
            rgbGreen: 0,
            rgbRed: 0,
            rgbReserved: 0,
        }],
    },
    memory: null_mut(),
    bytes_per_pixel: 0,
    width: 0,
    height: 0,
});

pub struct OffscreenBuffer {
    info: BITMAPINFO,
    memory: *mut c_void,
    width: i32,
    height: i32,
    bytes_per_pixel: i32,
}

impl OffscreenBuffer {
    #[allow(unused)]
    pub fn pitch(&self) -> isize {
        (self.width * self.bytes_per_pixel) as isize
    }

    #[allow(unused)]
    pub fn memory_size(&self) -> i32 {
        self.bytes_per_pixel * self.width * self.height
    }
}

unsafe fn window_dimension(window: HWND) -> (i32, i32) {
    let mut client_rect = RECT::default();
    GetClientRect(window, &mut client_rect).expect("Failed to get drawing window rect!");

    let width = client_rect.right - client_rect.left;
    let height = client_rect.bottom - client_rect.top;

    (width, height)
}

unsafe fn resize_dib_section(buffer: &mut OffscreenBuffer, width: i32, height: i32) {
    //TODO(voided): bulletproof this.
    //maybe don't free first, free after, then free first if that fails.

    if !buffer.memory.is_null() {
        VirtualFree(buffer.memory, 0, MEM_RELEASE).ok();
        buffer.memory = null_mut();
    }

    buffer.width = width;
    buffer.height = height;
    buffer.bytes_per_pixel = 4;

    buffer.info.bmiHeader.biSize = mem::size_of::<BITMAPINFOHEADER>() as u32;
    buffer.info.bmiHeader.biWidth = buffer.width;
    buffer.info.bmiHeader.biHeight = -buffer.height; // sets origin to top left
    buffer.info.bmiHeader.biPlanes = 1;
    buffer.info.bmiHeader.biBitCount = 32;
    buffer.info.bmiHeader.biCompression = BI_RGB.0;

    buffer.memory = VirtualAlloc(
        None,
        buffer.memory_size() as usize,
        MEM_COMMIT,
        PAGE_READWRITE,
    );
}

unsafe fn win32_display_buffer_in_window(
    buffer: &OffscreenBuffer,
    device_context: HDC,
    x: i32,
    y: i32,
    window_width: i32,
    window_height: i32,
) {
    //TODO(voided): Aspect ratio correction
    StretchDIBits(
        device_context,
        x,
        y,
        window_width,
        window_height,
        0,
        0,
        buffer.width,
        buffer.height,
        Some(buffer.memory),
        &buffer.info,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
}

/// # Safety
/// this is a system api call
pub unsafe extern "system" fn window_procedure(
    window: HWND,
    message: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    let mut result: LRESULT = LRESULT(0);

    match message {
        WM_QUIT => {
            RUN_STATE = RunState::Stopping;
        }
        WM_ACTIVATEAPP => {
            println!("WM_ACTIVATEAPP");
        }
        WM_SIZE => {
            println!("WM_SIZE");
        }
        WM_CLOSE => {
            println!("WM_CLOSE");
            //TODO(voided): Handle with message to the user?
            RUN_STATE = RunState::Stopping;
        }
        WM_DESTROY => {
            println!("WM_DESTROY");
            //TODO(voided): Handle this as error - recreate the window?
            RUN_STATE = RunState::Stopping;
        }

        WM_PAINT => {
            println!("WM_PAINT");

            let (window_width, window_height) = window_dimension(window);

            let mut paint: PAINTSTRUCT = PAINTSTRUCT::default();
            let hdc = BeginPaint(window, &mut paint);

            let x = paint.rcPaint.left;
            let y = paint.rcPaint.top;

            win32_display_buffer_in_window(
                &GLOBAL_BACK_BUFFER,
                hdc,
                x,
                y,
                window_width,
                window_height,
            );
            EndPaint(window, &paint);
        }
        _ => {
            result = DefWindowProcW(window, message, w_param, l_param);
        }
    }

    result
}

fn win32_process_x_input_digital_button(
    input: &XInputGamepad,
    button: u16,
    old_state: &ButtonInput,
    new_state: &mut ButtonInput,
) {
    new_state.button_is_down = has_bitflag!(input.wButtons, button);
    new_state.half_transitions = if old_state.button_is_down != new_state.button_is_down {
        1
    } else {
        0
    };
}

fn normalize_i16(x: i16) -> f32 {
    x as f32 / i16::MAX as f32
}

fn win32_process_stick(stick_position: i16, stick_deadzone: f32) -> f32 {
    let position_normalized = normalize_i16(stick_position);

    if position_normalized.abs() >= stick_deadzone {
        position_normalized.signum() * (position_normalized.abs() - stick_deadzone)
            / (1.0 - stick_deadzone)
    } else {
        0.0
    }
}

unsafe fn win32_process_pending_messages(keyboard_controller: &mut GameControllerInput) {
    let mut message = Default::default();
    while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
        match message.message {
            WM_SYSKEYDOWN | WM_SYSKEYUP | WM_KEYDOWN | WM_KEYUP => {
                const KEY_ALT_IS_DOWN_FLAG: u32 = 1 << 29;
                const KEY_PREVIOUS_DOWN_FLAG: u32 = 1 << 30;
                const KEY_IS_UP_FLAG: u32 = 1 << 31;

                let l_param = message.lParam.0 as u32;

                let vk_code = VIRTUAL_KEY(message.wParam.0 as _);
                let key_was_down = has_bitflag!(l_param, KEY_PREVIOUS_DOWN_FLAG);
                let key_is_down = !has_bitflag!(l_param, KEY_IS_UP_FLAG);
                let key_alt_is_down = has_bitflag!(l_param, KEY_ALT_IS_DOWN_FLAG);

                if key_is_down != key_was_down {
                    match vk_code {
                        VK_W => {
                            win32_process_keyboard_message(
                                &mut keyboard_controller.move_up,
                                key_is_down,
                            );
                        }
                        VK_A => {
                            win32_process_keyboard_message(
                                &mut keyboard_controller.move_left,
                                key_is_down,
                            );
                        }
                        VK_S => {
                            win32_process_keyboard_message(
                                &mut keyboard_controller.move_down,
                                key_is_down,
                            );
                        }
                        VK_D => {
                            win32_process_keyboard_message(
                                &mut keyboard_controller.move_right,
                                key_is_down,
                            );
                        }
                        VK_Q => {
                            win32_process_keyboard_message(
                                &mut keyboard_controller.shoulder_left,
                                key_is_down,
                            );
                        }
                        VK_E => {
                            win32_process_keyboard_message(
                                &mut keyboard_controller.shoulder_right,
                                key_is_down,
                            );
                        }
                        VK_UP => {
                            win32_process_keyboard_message(
                                &mut keyboard_controller.action_up,
                                key_is_down,
                            );
                        }
                        VK_DOWN => {
                            win32_process_keyboard_message(
                                &mut keyboard_controller.action_down,
                                key_is_down,
                            );
                        }
                        VK_LEFT => {
                            win32_process_keyboard_message(
                                &mut keyboard_controller.action_left,
                                key_is_down,
                            );
                        }
                        VK_RIGHT => {
                            win32_process_keyboard_message(
                                &mut keyboard_controller.action_right,
                                key_is_down,
                            );
                        }
                        VK_ESCAPE => {
                            win32_process_keyboard_message(
                                &mut keyboard_controller.start,
                                key_is_down,
                            );
                        }
                        VK_SPACE => {
                            win32_process_keyboard_message(
                                &mut keyboard_controller.back,
                                key_is_down,
                            );
                        }
                        _ => {}
                    }
                }

                if vk_code == VK_F4 && key_alt_is_down {
                    RUN_STATE = RunState::Stopping;
                }
            }
            _ => {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
}

fn win32_process_keyboard_message(new_button_state: &mut ButtonInput, key_is_down: bool) {
    debug_assert!(new_button_state.button_is_down != key_is_down);
    new_button_state.button_is_down = key_is_down;
    new_button_state.half_transitions += 1;
}

#[inline]
unsafe fn win32_get_wall_clock() -> i64 {
    let mut end_counter = Default::default();
    QueryPerformanceCounter(&mut end_counter).ok();
    end_counter
}

#[inline]
unsafe fn win32_get_elapsed_seconds(start: i64, end: i64) -> f32 {
    (end - start) as f32 / PERFORMANCE_FREQUENCY as f32
}

pub fn win32main() {
    unsafe {
        // sets windows scheduler time | sleep granularity
        let sleep_is_granular = timeBeginPeriod(1) == TIMERR_NOERROR;

        // TODO(voided): get actual get refresh rate reliably?
        let monitor_refresh_hz = 60;
        let game_update_hz = monitor_refresh_hz / 2;
        let target_seconds_per_frame = 1.0 / game_update_hz as f32;

        {
            let x_input = load_xinput();
            if x_input.is_none() {
                println!("Failed to load XINPUT. No controller support!");
            } else {
                println!("Loaded XINPUT. Controller support enabled!");
            }
        }

        let audio_source = BufferAudioSource::new(RingBuffer::with_default(48000 * 2));
        let audio_source = ThreadSharedAudioSource::new(audio_source);
        let _audio = AudioOutput::new(audio_source.clone(), target_seconds_per_frame);

        let mut sound_buffer_mem = vec![0.0; 48000 * 2];

        let mut game_memory = GameMemory {
            is_initalized: false,
            permanent_storage_size: megabytes(64),
            permanent_storage: VirtualAlloc(
                Some(null_mut()),
                megabytes(64),
                MEM_COMMIT,
                PAGE_READWRITE,
            ),
            transient_storage_size: gigabytes(4),
            transient_storage: VirtualAlloc(
                Some(null_mut()),
                gigabytes(4),
                MEM_COMMIT,
                PAGE_READWRITE,
            ),
        };

        resize_dib_section(&mut GLOBAL_BACK_BUFFER, 1280, 720);

        let instance = GetModuleHandleW(None).expect("failed to lodd instance");
        debug_assert!(instance.0 != 0);

        let window_class = w!("VoidedsHandmadeClass");

        let wc = WNDCLASSW {
            hCursor: Default::default(),
            hInstance: instance.into(),
            lpszClassName: window_class,

            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_procedure),
            ..Default::default()
        };

        let atom = RegisterClassW(&wc);
        debug_assert!(atom != 0);

        let window = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            window_class,
            w!("Voideds Handmade?"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            instance,
            None,
        );

        RUN_STATE = RunState::Running;
        let mut last_rdtsc = _rdtsc();

        QueryPerformanceFrequency(&mut PERFORMANCE_FREQUENCY).ok();

        let mut last_counter = win32_get_wall_clock();

        let mut old_inputs = GameInput::default();
        let mut new_inputs = GameInput::default();

        if !game_memory.permanent_storage.is_null() {
            while RUN_STATE != RunState::Stopping {
                new_inputs[0] = Default::default();
                let new_keyboard_controller = &mut new_inputs[0];
                let old_keyboard_controller = &mut old_inputs[0];

                for (new_button, old_button) in new_keyboard_controller
                    .buttons_mut()
                    .into_iter()
                    .zip(old_keyboard_controller.buttons().into_iter())
                {
                    new_button.button_is_down = old_button.button_is_down;
                }

                new_keyboard_controller.is_connected = true;

                win32_process_pending_messages(new_keyboard_controller);

                //TODO(voided): Update to a more modern api.
                //TODO(voided): Test how to dynamically load XInput in case it's not available. (day 6 - 22:00)
                //TODO(voided): Should we poll this more frequently.
                let max_controllers = max(new_inputs.len() as u32 - 1, XUSER_MAX_COUNT);
                let mut controller_state = XinputState::default();
                for controller_index in 0..max_controllers {
                    let old_input = &mut old_inputs[controller_index as usize + 1];
                    let new_input = &mut new_inputs[controller_index as usize + 1];

                    let result = XINPUT_GET_STATE(controller_index, &mut controller_state);

                    if result == ERROR_SUCCESS.0 {
                        //Note(voided): Controller is plugged in.
                        new_input.is_connected = true;
                        //TODO(voided): See if controller_state.dwPacketNumber increments too rapidly.

                        let gamepad = &controller_state.Gamepad;
                        #[allow(unused)]
                        let keypad_up = has_bitflag!(gamepad.wButtons, XINPUT_GAMEPAD_DPAD_UP);
                        #[allow(unused)]
                        let keypad_down = has_bitflag!(gamepad.wButtons, XINPUT_GAMEPAD_DPAD_DOWN);
                        #[allow(unused)]
                        let keypad_left = has_bitflag!(gamepad.wButtons, XINPUT_GAMEPAD_DPAD_LEFT);
                        #[allow(unused)]
                        let keypad_right =
                            has_bitflag!(gamepad.wButtons, XINPUT_GAMEPAD_DPAD_RIGHT);

                        const STICK_DEADZONE: f32 = 0.1;
                        const THRESHOLD: f32 = 0.5;

                        new_input.stick_left.x_average =
                            win32_process_stick(gamepad.sThumbLX, STICK_DEADZONE);
                        new_input.stick_left.y_average =
                            win32_process_stick(gamepad.sThumbLY, STICK_DEADZONE);

                        if new_input.stick_left.x_average != 0.0
                            || new_input.stick_left.y_average != 0.0
                        {
                            new_input.is_analog = true;
                        }

                        if has_bitflag!(gamepad.wButtons, XINPUT_GAMEPAD_DPAD_UP) {
                            new_input.stick_left.y_average = 1.0;
                            new_input.is_analog = false;
                        } else if has_bitflag!(gamepad.wButtons, XINPUT_GAMEPAD_DPAD_DOWN) {
                            new_input.stick_left.y_average = -1.0;
                            new_input.is_analog = false;
                        }

                        if has_bitflag!(gamepad.wButtons, XINPUT_GAMEPAD_DPAD_LEFT) {
                            new_input.stick_left.x_average = -1.0;
                            new_input.is_analog = false;
                        } else if has_bitflag!(gamepad.wButtons, XINPUT_GAMEPAD_DPAD_RIGHT) {
                            new_input.stick_left.x_average = 1.0;
                            new_input.is_analog = false;
                        }

                        let mut fake_button_inputs: XInputGamepad = Default::default();
                        const FAKE_BUTTON_UP: u16 = 1;
                        const FAKE_BUTTON_DOWN: u16 = 2;
                        const FAKE_BUTTON_LEFT: u16 = 4;
                        const FAKE_BUTTON_RIGHT: u16 = 8;

                        if new_input.stick_left.y_average >= THRESHOLD {
                            fake_button_inputs.wButtons += FAKE_BUTTON_UP;
                        } else if new_input.stick_left.y_average.abs() >= THRESHOLD {
                            fake_button_inputs.wButtons += FAKE_BUTTON_DOWN;
                        }

                        if new_input.stick_left.x_average >= THRESHOLD {
                            fake_button_inputs.wButtons += FAKE_BUTTON_RIGHT;
                        } else if new_input.stick_left.x_average.abs() >= THRESHOLD {
                            fake_button_inputs.wButtons += FAKE_BUTTON_LEFT;
                        }

                        win32_process_x_input_digital_button(
                            &fake_button_inputs,
                            FAKE_BUTTON_UP,
                            &old_input.move_up,
                            &mut new_input.move_up,
                        );
                        win32_process_x_input_digital_button(
                            &fake_button_inputs,
                            FAKE_BUTTON_DOWN,
                            &old_input.move_down,
                            &mut new_input.move_down,
                        );
                        win32_process_x_input_digital_button(
                            &fake_button_inputs,
                            FAKE_BUTTON_LEFT,
                            &old_input.move_left,
                            &mut new_input.move_left,
                        );
                        win32_process_x_input_digital_button(
                            &fake_button_inputs,
                            FAKE_BUTTON_RIGHT,
                            &old_input.move_right,
                            &mut new_input.move_right,
                        );

                        win32_process_x_input_digital_button(
                            gamepad,
                            XINPUT_GAMEPAD_START,
                            &old_input.start,
                            &mut new_input.start,
                        );
                        win32_process_x_input_digital_button(
                            gamepad,
                            XINPUT_GAMEPAD_BACK,
                            &old_input.back,
                            &mut new_input.back,
                        );
                        win32_process_x_input_digital_button(
                            gamepad,
                            XINPUT_GAMEPAD_Y,
                            &old_input.action_up,
                            &mut new_input.action_up,
                        );
                        win32_process_x_input_digital_button(
                            gamepad,
                            XINPUT_GAMEPAD_A,
                            &old_input.action_down,
                            &mut new_input.action_down,
                        );
                        win32_process_x_input_digital_button(
                            gamepad,
                            XINPUT_GAMEPAD_X,
                            &old_input.action_left,
                            &mut new_input.action_left,
                        );
                        win32_process_x_input_digital_button(
                            gamepad,
                            XINPUT_GAMEPAD_B,
                            &old_input.action_right,
                            &mut new_input.action_right,
                        );
                        win32_process_x_input_digital_button(
                            gamepad,
                            XINPUT_GAMEPAD_LEFT_SHOULDER,
                            &old_input.shoulder_left,
                            &mut new_input.shoulder_left,
                        );
                        win32_process_x_input_digital_button(
                            gamepad,
                            XINPUT_GAMEPAD_RIGHT_SHOULDER,
                            &old_input.shoulder_right,
                            &mut new_input.shoulder_right,
                        );
                    } else {
                        //Note(Voided): Controller is not available.
                        new_input.is_connected = false;
                    }
                }

                // game_update_and_render();
                let mut buffer = GameOffscreenBuffer {
                    memory: GLOBAL_BACK_BUFFER.memory,
                    width: GLOBAL_BACK_BUFFER.width,
                    height: GLOBAL_BACK_BUFFER.height,
                    bytes_per_pixel: GLOBAL_BACK_BUFFER.bytes_per_pixel,
                };

                let s = audio_source.source();
                let mut s = s.lock().expect("failed to lock source");
                let rate = s.sample_rate().unwrap_or(0);
                let to_fill = ((rate as usize / 15) * 2) - s.buffer.len();
                let sound_buffer_mem = &mut sound_buffer_mem[0..to_fill];
                let mut sound_buffer = GameSoundBuffer {
                    buffer: sound_buffer_mem,
                    samples_rate: rate,
                };

                game_update_and_render(
                    &mut game_memory,
                    &new_inputs,
                    &mut buffer,
                    &mut sound_buffer,
                );

                let (l, r) = s.buffer.write_buffers(to_fill);
                for (t, s) in l
                    .iter_mut()
                    .chain(r.iter_mut())
                    .zip(sound_buffer_mem.iter())
                {
                    *t = *s;
                }

                mem::swap(&mut old_inputs, &mut new_inputs);

                let work_counter = win32_get_wall_clock();
                let work_seconds_elapsed = win32_get_elapsed_seconds(last_counter, work_counter);

                let mut seconds_elapsed_this_frame = work_seconds_elapsed;

                if seconds_elapsed_this_frame < target_seconds_per_frame {
                    let time_to_wait = target_seconds_per_frame - seconds_elapsed_this_frame;
                    if sleep_is_granular {
                        std::thread::sleep(Duration::from_millis((time_to_wait / 1000.0) as u64));
                        seconds_elapsed_this_frame =
                            win32_get_elapsed_seconds(last_counter, win32_get_wall_clock());
                    }

                    debug_assert!(
                        win32_get_elapsed_seconds(last_counter, win32_get_wall_clock())
                            < target_seconds_per_frame
                    );

                    while seconds_elapsed_this_frame < target_seconds_per_frame {
                        seconds_elapsed_this_frame =
                            win32_get_elapsed_seconds(last_counter, win32_get_wall_clock());
                    }
                } else {
                    //TODO(voided): logging
                };

                let end_counter = win32_get_wall_clock();
                let ms_per_frame = win32_get_elapsed_seconds(last_counter, end_counter) * 1000.0;
                let fps = 1.0 / win32_get_elapsed_seconds(last_counter, end_counter);

                let end_performance_counter = win32_get_wall_clock();

                let end_rdtsc = _rdtsc();

                let cycles_elapsed = end_rdtsc - last_rdtsc;
                last_rdtsc = end_rdtsc;
                let mega_cycles_per_frame = cycles_elapsed as f32 / 1_000_000.0;

                last_counter = end_performance_counter;
                println!("{fps} f\\s\t {ms_per_frame} ms\\f\t {mega_cycles_per_frame} mc\\f");

                let device_context = GetDC(window);

                let (window_width, window_height) = window_dimension(window);

                win32_display_buffer_in_window(
                    &GLOBAL_BACK_BUFFER,
                    device_context,
                    0,
                    0,
                    window_width,
                    window_height,
                );

                ReleaseDC(window, device_context);
            }
        }
    }
}
