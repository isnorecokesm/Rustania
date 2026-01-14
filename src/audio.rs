use rodio::{Decoder, OutputStreamHandle, Sink, Source};
use std::sync::{Arc, Mutex};

pub struct AudioSystem {
    stream_handle: OutputStreamHandle,
    hit_sound: Option<Arc<Vec<u8>>>,
    slider_sound: Option<Arc<Vec<u8>>>,
    active_slider_sinks: Arc<Mutex<Vec<Sink>>>,
}

impl AudioSystem {
    pub fn new(stream_handle: OutputStreamHandle) -> Self {
        // Try to load hit.wav and slider.wav from current directory
        let hit_sound = Self::load_sound("hit.wav");
        let slider_sound = Self::load_sound("slider.wav");

        if hit_sound.is_none() {
            eprintln!("Warning: hit.wav not found. Hit sounds will be silent.");
        }
        if slider_sound.is_none() {
            eprintln!("Warning: slider.wav not found. Slider sounds will be silent.");
        }

        Self {
            stream_handle,
            hit_sound,
            slider_sound,
            active_slider_sinks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn load_sound(path: &str) -> Option<Arc<Vec<u8>>> {
        match std::fs::read(path) {
            Ok(data) => Some(Arc::new(data)),
            Err(_) => None,
        }
    }

    /// Play hit sound (for regular notes and LN heads/tails)
    pub fn play_hit(&self) {
        if let Some(sound_data) = &self.hit_sound {
            let data = sound_data.clone();
            let handle = self.stream_handle.clone();

            // Spawn in separate thread to not block gameplay
            std::thread::spawn(move || {
                let cursor = std::io::Cursor::new((*data).clone());
                if let Ok(source) = Decoder::new(cursor) {
                    let _ = handle.play_raw(source.convert_samples::<f32>());

                }
            });
        }
    }

    /// Start playing slider sound (looped) for a specific lane
    pub fn play_slider_start(&self, _lane: usize) {
        if let Some(sound_data) = &self.slider_sound {
            let data = sound_data.clone();

            if let Ok(sink) = Sink::try_new(&self.stream_handle) {
                let cursor = std::io::Cursor::new((*data).clone());
                if let Ok(source) = Decoder::new(cursor) {
                    // Repeat indefinitely
                    sink.append(source.convert_samples::<f32>().repeat_infinite());


                    // Store the sink so we can stop it later
                    if let Ok(mut sinks) = self.active_slider_sinks.lock() {
                        sinks.push(sink);
                    }
                }
            }
        }
    }

    /// Stop all active slider sounds
    pub fn stop_slider(&self, _lane: usize) {
        // For simplicity, stop the oldest active sink
        if let Ok(mut sinks) = self.active_slider_sinks.lock() {
            if !sinks.is_empty() {
                let sink = sinks.remove(0);
                sink.stop();
            }
        }
    }

    /// Stop all slider sounds (for pause, song end, etc)
    pub fn stop_all_sliders(&self) {
        if let Ok(mut sinks) = self.active_slider_sinks.lock() {
            for sink in sinks.drain(..) {
                sink.stop();
            }
        }
    }
}
