use std::ffi::c_void;
use utility::audio::SineAudioSource;

pub struct GameMemory {
    pub is_initalized: bool,
    pub permanent_storage_size: usize,
    pub permanent_storage: *mut c_void, // init to 0
    pub transient_storage_size: usize,
    pub transient_storage: *mut c_void,
}

#[derive(Debug)]
pub struct GameState {
    pub tone: SineAudioSource,
    pub x_offset: i32,
    pub y_offset: i32,
}

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

pub type GameInput = [GameControllerInput; 5];

#[derive(Default, Debug)]
pub struct GameControllerInput {
    pub is_analog: bool,
    pub is_connected: bool,

    pub stick_left: StickInput,
    pub move_up: ButtonInput,
    pub move_down: ButtonInput,
    pub move_left: ButtonInput,
    pub move_right: ButtonInput,
    pub action_up: ButtonInput,
    pub action_down: ButtonInput,
    pub action_left: ButtonInput,
    pub action_right: ButtonInput,
    pub shoulder_left: ButtonInput,
    pub shoulder_right: ButtonInput,
    pub start: ButtonInput,
    pub back: ButtonInput,
}

impl GameControllerInput {
    pub fn buttons(&self) -> [&ButtonInput; 12] {
        [
            &self.move_up,
            &self.move_down,
            &self.move_left,
            &self.move_right,
            &self.action_up,
            &self.action_down,
            &self.action_left,
            &self.action_right,
            &self.shoulder_left,
            &self.shoulder_right,
            &self.start,
            &self.back,
        ]
    }

    pub fn buttons_mut(&mut self) -> [&mut ButtonInput; 12] {
        [
            &mut self.move_up,
            &mut self.move_down,
            &mut self.move_left,
            &mut self.move_right,
            &mut self.action_up,
            &mut self.action_down,
            &mut self.action_left,
            &mut self.action_right,
            &mut self.shoulder_left,
            &mut self.shoulder_right,
            &mut self.start,
            &mut self.back,
        ]
    }
}

#[derive(Default, Debug)]
pub struct ButtonInput {
    pub button_is_down: bool,
    pub half_transitions: u32,
}

#[derive(Default, Debug)]
pub struct StickInput {
    pub x_average: f32,
    pub y_average: f32,
}
