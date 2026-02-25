use rodio::{OutputStream, Sink, Source};
use std::sync::Arc;
use std::time::Duration;

/// Types of sounds that can be played.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SoundType {
    /// Success sound - code validated and added to buffer
    Success,
    /// Error sound - validation failed
    Error,
}

/// Audio player for feedback sounds.
///
/// Uses programmatically generated tones to avoid file dependencies.
pub struct AudioPlayer {
    /// Audio output stream (must be kept alive)
    _stream: OutputStream,
    /// Sink for playback
    _sink: Arc<Sink>,
}

impl AudioPlayer {
    /// Creates a new audio player.
    ///
    /// Returns None if audio output is not available.
    pub fn new() -> Option<Self> {
        let (stream, stream_handle) = OutputStream::try_default().ok()?;
        let sink = Sink::try_new(&stream_handle).ok()?;

        Some(Self {
            _stream: stream,
            _sink: Arc::new(sink),
        })
    }

    /// Plays a sound of the specified type.
    pub fn play(&self, sound_type: SoundType) {
        // Create a new output stream for each sound to allow overlapping
        if let Ok((_, handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&handle) {
                match sound_type {
                    SoundType::Success => {
                        // Two-tone ascending beep: 800Hz -> 1200Hz
                        let tone1 = SineWave::new(800.0)
                            .take_duration(Duration::from_millis(100))
                            .amplify(0.3);
                        let tone2 = SineWave::new(1200.0)
                            .take_duration(Duration::from_millis(100))
                            .amplify(0.3);
                        sink.append(tone1);
                        sink.append(tone2);
                    }
                    SoundType::Error => {
                        // Two-tone descending buzz: 400Hz -> 200Hz
                        let tone1 = SineWave::new(400.0)
                            .take_duration(Duration::from_millis(150))
                            .amplify(0.4);
                        let tone2 = SineWave::new(200.0)
                            .take_duration(Duration::from_millis(200))
                            .amplify(0.4);
                        sink.append(tone1);
                        sink.append(tone2);
                    }
                }
                sink.detach(); // Let it play without blocking
            }
        }
    }
}

/// Simple sine wave generator.
struct SineWave {
    freq: f32,
    sample_rate: u32,
    position: u32,
}

impl SineWave {
    fn new(freq: f32) -> Self {
        Self {
            freq,
            sample_rate: 44100,
            position: 0,
        }
    }
}

impl Iterator for SineWave {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let value = (2.0 * std::f32::consts::PI * self.freq * self.position as f32
            / self.sample_rate as f32)
            .sin();
        self.position = self.position.wrapping_add(1);
        Some(value)
    }
}

impl Source for SineWave {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
