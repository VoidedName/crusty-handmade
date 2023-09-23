use cpal::Stream;
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, Host};

use crate::ring_buffer::RingBuffer;

pub struct BufferAudioSource {
    pub buffer: RingBuffer<f32>,
    sample_rate: Option<u32>,
}

impl AudioSource for BufferAudioSource {
    fn sample(&mut self, sample_rate: u32) -> (f32, f32) {
        self.sample_rate = Some(sample_rate);
        let (l, r) = self.buffer.read(2);
        let mut iter = l.iter().chain(r.iter());
        let a = *iter.next().unwrap_or(&0.0);
        let b = *iter.next().unwrap_or(&0.0);
        (a, b)
    }
}

impl BufferAudioSource {
    pub fn new(buffer: RingBuffer<f32>) -> Self {
        Self {
            buffer,
            sample_rate: None,
        }
    }

    pub fn sample_rate(&self) -> Option<u32> {
        self.sample_rate
    }
}

#[derive(Debug)]
pub struct SineAudioSource {
    pub hz: u32,
    pub volume: f32,
    position: f32,
}

impl AudioSource for SineAudioSource {
    fn sample(&mut self, sample_rate: u32) -> (f32, f32) {
        //hz = periods per second
        //sample_rate = samples per second
        const PERIOD: f32 = 2.0 * PI;
        let rate = self.hz as f32 / sample_rate as f32;
        let step = rate * PERIOD;
        self.position = (self.position + step) % PERIOD;
        let value = self.position.sin() * self.volume;
        (value, value)
    }
}

impl SineAudioSource {
    #[allow(unused)]
    pub const fn new(hz: u32, volume: f32) -> Self {
        Self {
            hz,
            volume,
            position: 0.0,
        }
    }

    pub fn reset_with(&mut self, hz: u32, volume: f32) {
        self.hz = hz;
        self.volume = volume;
        self.position = 0.0;
    }
}

pub struct ThreadSharedAudioSource<Source: AudioSource> {
    inner_source: Arc<Mutex<Source>>,
}

impl<Source: AudioSource> Clone for ThreadSharedAudioSource<Source> {
    fn clone(&self) -> Self {
        Self {
            inner_source: self.inner_source.clone(),
        }
    }
}

impl<Source: AudioSource> AudioSource for ThreadSharedAudioSource<Source> {
    fn sample(&mut self, sample_rate: u32) -> (f32, f32) {
        self.inner_source
            .lock()
            .expect("failed to sample lock")
            .sample(sample_rate)
    }
}

impl<Source: AudioSource> ThreadSharedAudioSource<Source> {
    pub fn new(inner_source: Source) -> Self {
        Self {
            inner_source: Arc::new(Mutex::new(inner_source)),
        }
    }

    pub fn source(&self) -> Arc<Mutex<Source>> {
        self.inner_source.clone()
    }
}

pub trait AudioSource: Send {
    fn sample(&mut self, sample_rate: u32) -> (f32, f32);
}

#[allow(unused)]
pub struct AudioOutput {
    host: Host,
    pub sample_rate: u32,
    pub channels: u32,
    stream: Stream,
    buffer_duration: f32,
}

//TODO: Deal with disconnected audio etc

impl AudioOutput {
    pub fn new<Source: AudioSource + 'static>(
        source: ThreadSharedAudioSource<Source>,
        buffer_duration: f32,
    ) -> Self {
        let host = cpal::default_host();
        let source = source;
        let device = host
            .default_output_device()
            .expect("no output device available");

        let config = device
            .supported_output_configs()
            .unwrap()
            .find(|p| p.channels() == 2)
            .unwrap();

        let sample_rate = config.min_sample_rate();

        let mut config = config.with_sample_rate(sample_rate).config();

        let channels = config.channels.into();

        let buffers_size = ((sample_rate.0 * channels) as f32 * buffer_duration) as u32;

        config.buffer_size = BufferSize::Fixed(buffers_size);

        //TODO: deal with other output formats (i16 and u16)?

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _info| {
                    let sound = &mut source.inner_source.lock().expect("failed to lock sound");
                    for d in data.chunks_mut(2) {
                        let (l, r) = sound.sample(sample_rate.0);

                        d[0] = l;
                        d[1] = r;
                    }
                },
                |err| {
                    println!("{}", err);
                },
                Some(Duration::from_secs_f64(0.01)),
            )
            .unwrap();

        stream.play().unwrap();

        Self {
            host,
            sample_rate: sample_rate.0,
            stream,
            channels,
            buffer_duration,
        }
    }
}
