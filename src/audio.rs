pub struct AudioOutput {
    host: Host,
    pub buffer: Arc<Mutex<RingBuffer<f32>>>,
    pub sample_rate: u32,
    pub channels: u32,
    stream: Stream,
}

//TODO: Deal with disconnected audio etc

impl AudioOutput {
    pub fn new(playback_buffer_time: f32) -> Self {
        let host = cpal::default_host();

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

        config.buffer_size = BufferSize::Fixed(sample_rate.0 * 2);

        let channels = config.channels.into();

        let buffer = Arc::new(Mutex::new(RingBuffer::with_default(
            (sample_rate.0 as f32 * playback_buffer_time) as usize * 2,
        )));
        let internal_buffer = buffer.clone();

        //TODO: deal with other output formats (i16 and u16)?

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _info| {
                    let mut buffer = internal_buffer.lock().unwrap();
                    let (l, r) = buffer.read(data.len());

                    let read_data = l.into_iter().chain(r.into_iter());

                    for (sample, read) in data.iter_mut().zip(read_data) {
                        *sample = Sample::from_sample(*read);
                    }
                },
                |err| {
                    println!("{}", err);
                },
                None,
            )
            .unwrap();

        stream.play().unwrap();

        Self {
            host,
            buffer,
            sample_rate: sample_rate.0,
            stream,
            channels,
        }
    }
}

