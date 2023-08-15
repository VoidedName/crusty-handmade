#![allow(unused)]
#![allow(non_snake_case)]

use std::mem;

use windows::core::*;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

type BYTE = u8;
type SHORT = u16;
type WORD = u32;

pub const XINPUT_GAMEPAD_DPAD_UP: WORD = 0x0001;
pub const XINPUT_GAMEPAD_DPAD_DOWN: WORD = 0x0002;
pub const XINPUT_GAMEPAD_DPAD_LEFT: WORD = 0x0004;
pub const XINPUT_GAMEPAD_DPAD_RIGHT: WORD = 0x0008;
pub const XINPUT_GAMEPAD_START: WORD = 0x0010;
pub const XINPUT_GAMEPAD_BACK: WORD = 0x0020;
pub const XINPUT_GAMEPAD_LEFT_THUMB: WORD = 0x0040;
pub const XINPUT_GAMEPAD_RIGHT_THUMB: WORD = 0x0080;
pub const XINPUT_GAMEPAD_LEFT_SHOULDER: WORD = 0x0100;
pub const XINPUT_GAMEPAD_RIGHT_SHOULDER: WORD = 0x0200;
pub const XINPUT_GAMEPAD_A: WORD = 0x1000;
pub const XINPUT_GAMEPAD_B: WORD = 0x2000;
pub const XINPUT_GAMEPAD_X: WORD = 0x4000;
pub const XINPUT_GAMEPAD_Y: WORD = 0x8000;

pub const XUSER_MAX_COUNT: WORD = 4;

//noinspection ALL
///[`XINPUT_STATE`](https://learn.microsoft.com/en-us/windows/win32/api/xinput/ns-xinput-xinput_state)
#[repr(C)]
#[derive(Default)]
pub struct XINPUT_STATE {
    pub dwPacketNumber: WORD,
    pub Gamepad: XINPUT_GAMEPAD,
}

//noinspection ALL
///[`XINPUT_GAMEPAD`](https://learn.microsoft.com/en-us/windows/win32/api/xinput/ns-xinput-xinput_gamepad)
#[repr(C)]
#[derive(Default)]
pub struct XINPUT_GAMEPAD {
    pub wButtons: WORD,
    pub bLeftTrigger: BYTE,
    pub bRightTrigger: BYTE,
    pub sThumbLX: SHORT,
    pub sThumbLY: SHORT,
    pub sThumbRX: SHORT,
    pub sThumbRY: SHORT,
}

//noinspection ALL
///[`XINPUT_GAMEPAD`](https://learn.microsoft.com/en-us/windows/win32/api/xinput/ns-xinput-xinput_gamepad)
#[repr(C)]
#[derive(Default)]
pub struct XINPUT_VIBRATION {
    pub wLeftMotorSpeed: WORD,
    pub wRightMotorSpeed: WORD,
}

//noinspection ALL
pub struct XINPUT {
    ///[`XInputGetState`](https://learn.microsoft.com/en-us/windows/win32/api/xinput/nf-xinput-xinputgetstate)
    pub XInputGetState: unsafe fn(
        /* in: dwUserIndex */ WORD,
        /* out: pState */ *mut XINPUT_STATE,
    ) -> WORD,
    ///[`XInputSetState`](https://learn.microsoft.com/en-us/windows/win32/api/xinput/nf-xinput-xinputsetstate)
    pub XInputSetState: unsafe fn(
        /* in: dwUserIndex */ WORD,
        /* in out: pVibration */ *mut XINPUT_VIBRATION,
    ) -> WORD,
}

pub fn load_xinput() -> Option<XINPUT> {
    unsafe {
        if let Ok(module) = LoadLibraryW(w!("XInput1_4.dll")) {
            if !module.is_invalid() {
                let xinput_get_state = GetProcAddress(module, s!("XInputGetState"));
                let xinput_set_state = GetProcAddress(module, s!("XInputSetState"));

                //TODO(voided): Think about maybe going against a dummy instead of options or fail all.
                return if xinput_get_state.is_none() || xinput_set_state.is_none() {
                    None
                } else {
                    Some(XINPUT {
                        XInputGetState: mem::transmute(xinput_get_state.unwrap()),
                        XInputSetState: mem::transmute(xinput_set_state.unwrap()),
                    })
                };
            }
        }
        None
    }
}
