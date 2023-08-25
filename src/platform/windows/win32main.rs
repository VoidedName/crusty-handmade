use crate::game_update_and_render;
use crate::GameOffscreenBuffer;
use std::ffi::c_void;
use std::fmt::Debug;
use std::mem;
use std::ptr::null_mut;

use windows::Win32::System::Memory::{
    VirtualAlloc, VirtualFree, MEM_COMMIT, MEM_RELEASE, PAGE_READWRITE,
};
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::{
    core::*, Win32::Foundation::*, Win32::Graphics::Gdi::*,
    Win32::System::LibraryLoader::GetModuleHandleW, Win32::UI::WindowsAndMessaging::*,
};

use crate::audio::{AudioOutput, SineAudioSource, ThreadSharedAudioSource};
use crate::x_input::*;
use crate::global_mut;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum RunState {
    Starting,
    Running,
    Stopping,
}

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

unsafe fn display_buffer_in_window(
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

        WM_SYSKEYDOWN | WM_SYSKEYUP | WM_KEYDOWN | WM_KEYUP => {
            const KEY_ALT_IS_DOWN_FLAG: u32 = 1 << 29;
            const KEY_PREVIOUS_DOWN_FLAG: u32 = 1 << 30;
            const KEY_IS_UP_FLAG: u32 = 1 << 31;

            let l_param = l_param.0 as u32;

            let vk_code = VIRTUAL_KEY(w_param.0 as _);
            let key_was_down = l_param & KEY_PREVIOUS_DOWN_FLAG != 0;
            let key_is_down = l_param & KEY_IS_UP_FLAG == 0;
            let key_alt_is_down = l_param & KEY_ALT_IS_DOWN_FLAG != 0;

            if key_is_down != key_was_down {
                match vk_code {
                    VK_W => {
                        println!("W")
                    }
                    VK_A => {
                        println!("A")
                    }
                    VK_S => {
                        println!("S")
                    }
                    VK_D => {
                        println!("D")
                    }
                    VK_Q => {
                        println!("Q")
                    }
                    VK_E => {
                        println!("E")
                    }
                    VK_UP => {
                        println!("Up")
                    }
                    VK_DOWN => {
                        println!("Down")
                    }
                    VK_LEFT => {
                        println!("Left")
                    }
                    VK_RIGHT => {
                        println!("Right")
                    }
                    VK_ESCAPE => {
                        println!(
                            "Escape - key_is_down: {key_is_down} - key_was_down: {key_was_down}"
                        )
                    }
                    VK_SPACE => {
                        println!("Space")
                    }
                    _ => {}
                }
            }

            if vk_code == VK_F4 && key_alt_is_down {
                RUN_STATE = RunState::Stopping;
            }
        }

        WM_PAINT => {
            println!("WM_PAINT");

            let (window_width, window_height) = window_dimension(window);

            let mut paint: PAINTSTRUCT = PAINTSTRUCT::default();
            let hdc = BeginPaint(window, &mut paint);

            let x = paint.rcPaint.left;
            let y = paint.rcPaint.top;

            display_buffer_in_window(&GLOBAL_BACK_BUFFER, hdc, x, y, window_width, window_height);
            EndPaint(window, &paint);
        }
        _ => {
            result = DefWindowProcW(window, message, w_param, l_param);
        }
    }

    result
}


pub fn win32main() {
    unsafe {
        {
            let x_input = load_xinput();
            if x_input.is_none() {
                println!("Failed to load XINPUT. No controller support!");
            } else {
                println!("Loaded XINPUT. Controller support enabled!");
            }
        }

        let audio_source = SineAudioSource::new(255, 0.5);
        let audio_source = ThreadSharedAudioSource::new(audio_source);
        let _audio = AudioOutput::new(audio_source.clone());

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
        let mut message = MSG::default();
        let mut x_offset = 0;
        let mut y_offset = 0;
        let mut hz = 256;

        //let mut last_rdtsc = _rdtsc();

        let mut performance_frequency = Default::default();
        QueryPerformanceFrequency(&mut performance_frequency).ok();

        let mut last_performance_counter = Default::default();
        QueryPerformanceCounter(&mut last_performance_counter).ok();
        while RUN_STATE != RunState::Stopping {
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                if message.message == WM_QUIT {
                    RUN_STATE = RunState::Stopping;
                }

                TranslateMessage(&message);
                DispatchMessageW(&message);
            }

            //TODO(voided): Update to a more modern api.
            //TODO(voided): Test how to dynamically load XInput in case it's not available. (day 6 - 22:00)
            //TODO(voided): Should we poll this more frequently.

            let mut controller_state = XINPUT_STATE::default();
            for controller_index in 0..XUSER_MAX_COUNT {
                let result = XINPUT_GET_STATE(controller_index, &mut controller_state);
                if result == ERROR_SUCCESS.0 {
                    //Note(voided): Controller is plugged in.
                    //TODO(voided): See if controller_state.dwPacketNumber increments too rapidly.
                    let gamepad = &controller_state.Gamepad;

                    #[allow(unused)]
                    let keypad_up = (gamepad.wButtons & XINPUT_GAMEPAD_DPAD_UP) != 0;
                    #[allow(unused)]
                    let keypad_down = (gamepad.wButtons & XINPUT_GAMEPAD_DPAD_DOWN) != 0;
                    #[allow(unused)]
                    let keypad_left = (gamepad.wButtons & XINPUT_GAMEPAD_DPAD_LEFT) != 0;
                    #[allow(unused)]
                    let keypad_right = (gamepad.wButtons & XINPUT_GAMEPAD_DPAD_RIGHT) != 0;
                    #[allow(unused)]
                    let start = (gamepad.wButtons & XINPUT_GAMEPAD_START) != 0;
                    #[allow(unused)]
                    let back = (gamepad.wButtons & XINPUT_GAMEPAD_BACK) != 0;
                    #[allow(unused)]
                    let shoulder_left = (gamepad.wButtons & XINPUT_GAMEPAD_LEFT_SHOULDER) != 0;
                    #[allow(unused)]
                    let shoulder_right = (gamepad.wButtons & XINPUT_GAMEPAD_RIGHT_SHOULDER) != 0;
                    #[allow(unused)]
                    let button_a = (gamepad.wButtons & XINPUT_GAMEPAD_A) != 0;
                    #[allow(unused)]
                    let button_b = (gamepad.wButtons & XINPUT_GAMEPAD_B) != 0;
                    #[allow(unused)]
                    let button_x = (gamepad.wButtons & XINPUT_GAMEPAD_X) != 0;
                    #[allow(unused)]
                    let button_y = (gamepad.wButtons & XINPUT_GAMEPAD_Y) != 0;

                    #[allow(unused)]
                    let stick_x = gamepad.sThumbLX;
                    #[allow(unused)]
                    let stick_y = gamepad.sThumbLY;

                    //println!("{}, {} - {}, {}", gamepad.sThumbLX, gamepad.sThumbLY, gamepad.sThumbRX, gamepad.sThumbRY);

                    y_offset -= stick_y as i32 / 4096;
                    x_offset += stick_x as i32 / 4096;

                    hz =
                        (512 + (256.0 * (stick_y as f32 + stick_x as f32) / 60000.0) as i32) as u32;

                    // println!("{stick_y}, {stick_x}")
                } else {
                    //Note(Voided): Controller is not available.
                }
            }

            // game_update_and_render();
            let mut buffer = GameOffscreenBuffer {
                memory: GLOBAL_BACK_BUFFER.memory,
                width: GLOBAL_BACK_BUFFER.width,
                height: GLOBAL_BACK_BUFFER.height,
                bytes_per_pixel: GLOBAL_BACK_BUFFER.bytes_per_pixel,
            };
            game_update_and_render(&mut buffer, x_offset, y_offset);

            //TODO(voided) sound is very delayed
            audio_source
                .source()
                .lock()
                .expect("failed to lock source")
                .hz = hz;
            // println!("{sample_rate}, {bytes_to_write}, {}, {}", (0.2 * sample_rate as f32 * 2.0) as usize, buffer.len());

            let device_context = GetDC(window);
            let (window_width, window_height) = window_dimension(window);

            display_buffer_in_window(
                &GLOBAL_BACK_BUFFER,
                device_context,
                0,
                0,
                window_width,
                window_height,
            );

            ReleaseDC(window, device_context);

            // let end_rdtsc = _rdtsc();

            // let cycles_elapsed = end_rdtsc - last_rdtsc;
            // last_rdtsc = end_rdtsc;
            // let mega_cycles_per_frame = cycles_elapsed as f32 / 1_000_000.0;

            let mut end_performance_counter = 0;
            QueryPerformanceCounter(&mut end_performance_counter).ok();

            //let counter_elapsed = end_performance_counter - last_performance_counter;

            //let nanos_per_frame = (counter_elapsed as f32 * 1_000.0) / performance_frequency as f32;

            // let fps = performance_frequency as f32 / counter_elapsed as f32;

            // println!("{fps} f\\s\t {nanos_per_frame} ms\\f\t {mega_cycles_per_frame} mc\\f");

            // last_performance_counter = end_performance_counter;
        }
    }
}
