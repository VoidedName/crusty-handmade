#![allow(unused)]
#![allow(non_snake_case)]

use std::mem;

use crate::global_mut;
use windows::core::*;
use windows::Win32::Foundation::ERROR_DEVICE_NOT_CONNECTED;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

type Byte = u8;
type Short = i16;
type Word = u16;
type DWord = u32;

pub const XINPUT_GAMEPAD_DPAD_UP: Word = 0x0001;
pub const XINPUT_GAMEPAD_DPAD_DOWN: Word = 0x0002;
pub const XINPUT_GAMEPAD_DPAD_LEFT: Word = 0x0004;
pub const XINPUT_GAMEPAD_DPAD_RIGHT: Word = 0x0008;
pub const XINPUT_GAMEPAD_START: Word = 0x0010;
pub const XINPUT_GAMEPAD_BACK: Word = 0x0020;
pub const XINPUT_GAMEPAD_LEFT_THUMB: Word = 0x0040;
pub const XINPUT_GAMEPAD_RIGHT_THUMB: Word = 0x0080;
pub const XINPUT_GAMEPAD_LEFT_SHOULDER: Word = 0x0100;
pub const XINPUT_GAMEPAD_RIGHT_SHOULDER: Word = 0x0200;
pub const XINPUT_GAMEPAD_A: Word = 0x1000;
pub const XINPUT_GAMEPAD_B: Word = 0x2000;
pub const XINPUT_GAMEPAD_X: Word = 0x4000;
pub const XINPUT_GAMEPAD_Y: Word = 0x8000;

pub const XUSER_MAX_COUNT: DWord = 4;

//noinspection ALL
///[`XINPUT_STATE`](https://learn.microsoft.com/en-us/windows/win32/api/xinput/ns-xinput-xinput_state)
#[repr(C)]
#[derive(Default)]
pub struct XinputState {
    pub dwPacketNumber: DWord,
    pub Gamepad: XInputGamepad,
}

//noinspection ALL
///[`XINPUT_GAMEPAD`](https://learn.microsoft.com/en-us/windows/win32/api/xinput/ns-xinput-xinput_gamepad)
#[repr(C)]
#[derive(Default)]
pub struct XInputGamepad {
    pub wButtons: Word,
    pub bLeftTrigger: Byte,
    pub bRightTrigger: Byte,
    pub sThumbLX: Short,
    pub sThumbLY: Short,
    pub sThumbRX: Short,
    pub sThumbRY: Short,
}

//noinspection ALL
///[`XINPUT_GAMEPAD`](https://learn.microsoft.com/en-us/windows/win32/api/xinput/ns-xinput-xinput_gamepad)
#[repr(C)]
#[derive(Default)]
pub struct XInputVibration {
    pub wLeftMotorSpeed: Word,
    pub wRightMotorSpeed: Word,
}

///[`XInputSetState`](https://learn.microsoft.com/en-us/windows/win32/api/xinput/nf-xinput-xinputsetstate)
type XInputSetState__ = unsafe fn(
    /* in: dwUserIndex */ DWord,
    /* in out: pVibration */ *mut XInputVibration,
) -> DWord;
global_mut!(XINPUT_SET_STATE: XInputSetState__ = XInputSetState__Stub);
fn XInputSetState__Stub(_: DWord, _: *mut XInputVibration) -> DWord {
    ERROR_DEVICE_NOT_CONNECTED.0
}

///[`XInputGetState`](https://learn.microsoft.com/en-us/windows/win32/api/xinput/nf-xinput-xinputgetstate)
type XInputGetState__ =
    unsafe fn(/* in: dwUserIndex */ DWord, /* out: pState */ *mut XinputState) -> DWord;
global_mut!(XINPUT_GET_STATE: XInputGetState__ = XInputGetState__Stub);
fn XInputGetState__Stub(_: DWord, _: *mut XinputState) -> DWord {
    ERROR_DEVICE_NOT_CONNECTED.0
}

pub fn load_xinput() -> Option<()> {
    unsafe {
        let lib = LoadLibraryW(w!("XInput1_4.dll")).or_else(|_| LoadLibraryW(w!("XInput1_3.dll")));

        if let Ok(module) = lib {
            if !module.is_invalid() {
                let xinput_get_state = GetProcAddress(module, s!("XInputGetState"));
                let xinput_set_state = GetProcAddress(module, s!("XInputSetState"));

                //TODO(voided): Think about maybe going against a dummy instead of options or fail all.
                return if xinput_get_state.is_some() || xinput_set_state.is_some() {
                    XINPUT_GET_STATE = mem::transmute(xinput_get_state.unwrap());
                    XINPUT_SET_STATE = mem::transmute(xinput_set_state.unwrap());
                    Some(())
                } else {
                    None
                };
            }
        }
        None
    }
}
