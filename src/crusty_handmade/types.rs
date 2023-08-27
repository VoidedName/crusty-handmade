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

pub type GameInput = [GameControllerInput; 4];

#[derive(Default, Debug)]
pub struct GameControllerInput {
    pub is_analog: bool,
    pub button_up: ButtonInput,
    pub button_down: ButtonInput,
    pub button_left: ButtonInput,
    pub button_right: ButtonInput,
    pub button_shoulder_left: ButtonInput,
    pub button_shoulder_right: ButtonInput,
    pub stick_left: StickInput,
}

#[derive(Default, Debug)]
pub struct ButtonInput {
    pub button_is_down: bool,
    pub half_transitions: u32,
}

#[derive(Default, Debug)]
pub struct AnalogInput {
    pub start: f32,
    pub end: f32,
    pub min: f32,
    pub max: f32,
}

#[derive(Default, Debug)]
pub struct StickInput {
    pub x_axis: AnalogInput,
    pub y_axis: AnalogInput,
}
