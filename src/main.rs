use std::ffi::c_void;
use std::mem;
use std::ptr::null_mut;

use crate::x_input::*;
use windows::Win32::System::Memory::{
    VirtualAlloc, VirtualFree, MEM_COMMIT, MEM_RELEASE, PAGE_READWRITE,
};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::{
    core::*, Win32::Foundation::*, Win32::Graphics::Gdi::*,
    Win32::System::LibraryLoader::GetModuleHandleW, Win32::UI::WindowsAndMessaging::*,
};
use windows::Win32::Media::Audio::DirectSound::{DirectSoundCreate, DSBCAPS_PRIMARYBUFFER, DSBUFFERDESC, DSSCL_PRIORITY, IDirectSound};
use windows::Win32::Media::Audio::{WAVE_FORMAT_PCM, WAVEFORMATEX};

mod x_input;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum RunState {
    Starting,
    Running,
    Stopping,
}

///Declares a static mut! Allows to search for specifically global muts
macro_rules! global_mut {
    ($variable:ident : $t:ty = $e:expr) => {
        pub static mut $variable: $t = $e;
    };
}

use global_mut;

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
    pub fn pitch(&self) -> isize {
        (self.width * self.bytes_per_pixel) as isize
    }

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


unsafe fn init_d_sound(window: HWND, buffer_size: u32, samples_per_second: u32) {
    //TODO(voided): Replace with something newer.
    //Note(voided): Decided not to dynamically load this, cause the oop nature of it seems like a pain,
    // and i'm not willing to fight with it right now

    let mut d_sound: Option<IDirectSound> = None;
    match DirectSoundCreate(None, &mut d_sound, None) {
        Err(_) => {
            //TODO(voided): Diagnostics
        }
        Ok(_) => {
            match &d_sound {
                None => {
                    //TODO(voided): Diagnostics
                }
                Some(d_sound) => {
                    let channels: u16 = 2;
                    let bits_per_sample: u16 = 16;
                    let mut wave_format = WAVEFORMATEX {
                        wFormatTag: WAVE_FORMAT_PCM as _,
                        nChannels: channels,
                        nSamplesPerSec: samples_per_second,
                        nAvgBytesPerSec: samples_per_second * channels as u32 * bits_per_sample as u32 / 8,
                        wBitsPerSample: bits_per_sample as _,
                        nBlockAlign: channels * bits_per_sample / 8,
                        ..Default::default()
                    };

                    match d_sound.SetCooperativeLevel(window, DSSCL_PRIORITY) {
                        Err(_) => {
                            //TODO(voided): Diagnostics
                        }
                        Ok(_) => {
                            //NOTE(voided): "Create" a primary buffer.
                            let primary_buffer_description = DSBUFFERDESC {
                                dwSize: mem::size_of::<DSBUFFERDESC>() as u32,
                                dwFlags: DSBCAPS_PRIMARYBUFFER,
                                ..Default::default()
                            };
                            let mut primary_buffer = None;
                            match d_sound.CreateSoundBuffer(&primary_buffer_description, &mut primary_buffer, None) {
                                Err(..) => {
                                    //TODO(voided): Diagnostics
                                }
                                Ok(_) => {
                                    match &primary_buffer {
                                        None => {
                                            //TODO(voided): Diagnostics
                                        }
                                        Some(primary_buffer) => {
                                            match primary_buffer.SetFormat(&wave_format) {
                                                Err(..) => {
                                                    //TODO(voided): Diagnostics
                                                }
                                                Ok(_) => {
                                                    println!("{}", "primary buffer created successfully");
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let mut secondary_buffer = None;
                    let secondary_buffer_description = DSBUFFERDESC {
                        dwSize: mem::size_of::<DSBUFFERDESC>() as u32,
                        dwBufferBytes: buffer_size,
                        lpwfxFormat: &mut wave_format,
                        ..Default::default()
                    };
                    match d_sound.CreateSoundBuffer(&secondary_buffer_description, &mut secondary_buffer, None) {
                        Err(..) => {
                            //TODO(voided): Diagnostics
                        }
                        Ok(_) => {
                            match &secondary_buffer {
                                None => {
                                    //TODO(voided): Diagnostics
                                }
                                Some(secondary_buffer) => {
                                    println!("{}", "secondary buffer created successfully");
                                }
                            }
                        }
                    }
                }

                //NOTE(voided): "Create" a secondary buffer.

                //NOTE(voided): Start it playing!
            }
        }
    }
}

unsafe fn render_weird_gradient(buffer: &OffscreenBuffer, x_offset: i32, y_offest: i32) {
    // windows rgb order is actually padding-bgr
    // on lil endian we need to load xxRRGGBB
    let mut row = buffer.memory.cast::<u8>();
    let pitch = buffer.pitch();

    for y in 0..buffer.height {
        let mut pixel = row.cast::<u32>();
        for x in 0..buffer.width {
            let blue = (x + x_offset) as u32 & 0xFF;
            let green = (y + y_offest) as u32 & 0xFF;
            let a = green << 8 | blue;
            *pixel = a;
            pixel = pixel.offset(1);
        }
        row = row.offset(pitch);
    }
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

fn main() -> Result<()> {
    unsafe {
        {
            let x_input = load_xinput();
            if let None = x_input {
                println!("Failed to load XINPUT. No controller support!");
            } else {
                println!("Loaded XINPUT. Controller support enabled!");
            }
        }

        resize_dib_section(&mut GLOBAL_BACK_BUFFER, 1280, 720);

        let instance = GetModuleHandleW(None)?;
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

        init_d_sound(window, 48000 * mem::size_of::<i16>() as u32 * 2, 48000);

        RUN_STATE = RunState::Running;
        let mut message = MSG::default();
        let mut x_offset = 0;
        let mut y_offset = 0;

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
            loop {
                for controller_index in 0..XUSER_MAX_COUNT {
                    let result = XInputGetState(controller_index, &mut controller_state);
                    if result == ERROR_SUCCESS.0 {
                        //Note(voided): Controller is plugged in.
                        //TODO(voided): See if controller_state.dwPacketNumber increments too rapidly.
                        let gamepad = &controller_state.Gamepad;

                        let keypad_up = (gamepad.wButtons & XINPUT_GAMEPAD_DPAD_UP) != 0;
                        let keypad_down = (gamepad.wButtons & XINPUT_GAMEPAD_DPAD_DOWN) != 0;
                        let keypad_left = (gamepad.wButtons & XINPUT_GAMEPAD_DPAD_LEFT) != 0;
                        let keypad_right = (gamepad.wButtons & XINPUT_GAMEPAD_DPAD_RIGHT) != 0;
                        let start = (gamepad.wButtons & XINPUT_GAMEPAD_START) != 0;
                        let back = (gamepad.wButtons & XINPUT_GAMEPAD_BACK) != 0;
                        let shoulder_left =
                            (gamepad.wButtons & XINPUT_GAMEPAD_LEFT_SHOULDER) != 0;
                        let shoulder_right =
                            (gamepad.wButtons & XINPUT_GAMEPAD_RIGHT_SHOULDER) != 0;
                        let button_a = (gamepad.wButtons & XINPUT_GAMEPAD_A) != 0;
                        let button_b = (gamepad.wButtons & XINPUT_GAMEPAD_B) != 0;
                        let button_x = (gamepad.wButtons & XINPUT_GAMEPAD_X) != 0;
                        let button_y = (gamepad.wButtons & XINPUT_GAMEPAD_Y) != 0;

                        let stick_x = gamepad.sThumbLX;
                        let stick_y = gamepad.sThumbLY;

                        if button_a {
                            y_offset += 1;
                        }
                    } else {
                        //Note(Voided): Controller is not available.
                    }
                }

                break;
            }

            render_weird_gradient(&GLOBAL_BACK_BUFFER, x_offset, y_offset);

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

            x_offset += 1;
        }

        Ok(())
    }
}
