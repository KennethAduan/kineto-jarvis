use crate::domain::speech::{AudioPlayback, SpeechAudio};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, SampleFormat, SizedSample,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub struct SystemAudioPlayback(pub Arc<super::echo::RenderReference>);
struct Cursor {
    samples: Vec<f32>,
    position: f64,
    step: f64,
}
impl Cursor {
    fn next(&mut self) -> Option<f32> {
        let left = self.position as usize;
        if left >= self.samples.len() {
            return None;
        }
        let right = (left + 1).min(self.samples.len() - 1);
        let fraction = (self.position - left as f64) as f32;
        let value = self.samples[left] * (1. - fraction) + self.samples[right] * fraction;
        self.position += self.step;
        Some(value.clamp(-1., 1.))
    }
}
fn stream<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    audio: SpeechAudio,
    cancel: CancellationToken,
    done: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    reference: Arc<super::echo::RenderReference>,
) -> Result<cpal::Stream, String> {
    let channels = usize::from(config.channels);
    let mut render = super::echo::RenderFeed::new(reference, config.sample_rate);
    let mut cursor = Cursor {
        samples: audio.samples,
        position: 0.,
        step: f64::from(audio.sample_rate) / f64::from(config.sample_rate),
    };
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                for frame in data.chunks_mut(channels) {
                    let value = if cancel.is_cancelled() {
                        done.store(true, Ordering::Relaxed);
                        0.
                    } else {
                        cursor.next().unwrap_or_else(|| {
                            done.store(true, Ordering::Relaxed);
                            0.
                        })
                    };
                    frame.fill(T::from_sample(value));
                    render.push(value);
                }
            },
            move |_| {
                failed.store(true, Ordering::Relaxed);
            },
            None,
        )
        .map_err(|_| "Could not open the default speakers. Check system sound settings.".into())
}
impl AudioPlayback for SystemAudioPlayback {
    fn play(&self, audio: SpeechAudio, cancel: CancellationToken) -> Result<(), String> {
        if cancel.is_cancelled() {
            return Ok(());
        }
        if !(8_000..=48_000).contains(&audio.sample_rate)
            || audio.samples.is_empty()
            || audio.samples.len() > audio.sample_rate as usize * 60
            || audio.samples.iter().any(|s| !s.is_finite())
        {
            return Err("Invalid speech audio".into());
        }
        let device = cpal::default_host()
            .default_output_device()
            .ok_or("No audio output device found")?;
        let supported = device
            .default_output_config()
            .map_err(|_| "Could not read speaker format")?;
        let config: cpal::StreamConfig = supported.into();
        if config.channels == 0
            || config.channels > 8
            || !(8_000..=192_000).contains(&config.sample_rate)
        {
            return Err("Unsupported speaker format".into());
        }
        let done = Arc::new(AtomicBool::new(false));
        let failed = Arc::new(AtomicBool::new(false));
        let output = match supported.sample_format() {
            SampleFormat::F32 => stream::<f32>(
                &device,
                config,
                audio,
                cancel.clone(),
                done.clone(),
                failed.clone(),
                self.0.clone(),
            )?,
            SampleFormat::I16 => stream::<i16>(
                &device,
                config,
                audio,
                cancel.clone(),
                done.clone(),
                failed.clone(),
                self.0.clone(),
            )?,
            SampleFormat::U16 => stream::<u16>(
                &device,
                config,
                audio,
                cancel.clone(),
                done.clone(),
                failed.clone(),
                self.0.clone(),
            )?,
            SampleFormat::I32 => stream::<i32>(
                &device,
                config,
                audio,
                cancel.clone(),
                done.clone(),
                failed.clone(),
                self.0.clone(),
            )?,
            _ => return Err("Unsupported speaker sample format".into()),
        };
        output
            .play()
            .map_err(|_| "Could not start speech playback")?;
        let deadline = Instant::now() + Duration::from_secs(65);
        while !done.load(Ordering::Relaxed) && !cancel.is_cancelled() {
            if failed.load(Ordering::Relaxed) {
                return Err("Audio output disconnected during playback".into());
            }
            if Instant::now() > deadline {
                return Err("Speech playback timed out".into());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        // Let the device drain the last small output buffer; cancellation mutes the callback.
        for _ in 0..10 {
            if cancel.is_cancelled() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        drop(output);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "audible hardware check; requires KOKORO_TEST_DIR and VOICE_PLAYBACK_TEST=1; never records"]
    fn plays_generated_preview_through_default_output() {
        assert_eq!(std::env::var("VOICE_PLAYBACK_TEST").as_deref(), Ok("1"));
        let file =
            std::path::PathBuf::from(std::env::var_os("KOKORO_TEST_DIR").expect("test directory"))
                .join("george.wav");
        let mut reader = hound::WavReader::open(file).unwrap();
        let rate = reader.spec().sample_rate;
        let samples = reader
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / 65536.)
            .collect();
        SystemAudioPlayback::default()
            .play(
                SpeechAudio {
                    samples,
                    sample_rate: rate,
                },
                CancellationToken::new(),
            )
            .unwrap();
    }
}
