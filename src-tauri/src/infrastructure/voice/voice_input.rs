use crate::domain::voice_mode::{InputFrames, VoiceInput};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SampleFormat, SizedSample,
};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
    time::Duration,
};
// Eight 32 ms packets bound capture latency to roughly a quarter second.
// Backpressure drops stale packets and marks a discontinuity, never kills the device stream.
const CAPTURE_PACKETS: usize = 8;
#[derive(Default)]
struct CaptureQueue {
    frames: Mutex<VecDeque<Vec<f32>>>,
    ready: Condvar,
    discontinuity: AtomicBool,
}
impl CaptureQueue {
    fn push(&self, frame: Vec<f32>) {
        if let Ok(mut frames) = self.frames.try_lock() {
            if frames.len() == CAPTURE_PACKETS {
                frames.pop_front();
                self.discontinuity.store(true, Ordering::Relaxed);
            }
            frames.push_back(frame);
            self.ready.notify_one();
        } else {
            self.discontinuity.store(true, Ordering::Relaxed);
        }
    }
    fn pop(&self, timeout: Duration) -> Result<Option<Vec<f32>>, String> {
        let frames = self
            .frames
            .lock()
            .map_err(|_| "Microphone buffer unavailable")?;
        let (mut frames, _) = self
            .ready
            .wait_timeout_while(frames, timeout, |frames| frames.is_empty())
            .map_err(|_| "Microphone buffer unavailable")?;
        Ok(frames.pop_front())
    }
}
pub struct NativeVoiceInput(pub Arc<super::echo::RenderReference>);
struct Frames {
    _stream: cpal::Stream,
    queue: Arc<CaptureQueue>,
    discontinuity: AtomicBool,
    failed: Arc<AtomicBool>,
    echo: std::sync::Mutex<super::echo::CaptureEcho>,
}
impl InputFrames for Frames {
    fn next(&self, timeout: Duration) -> Result<Option<Vec<f32>>, String> {
        if self.failed.load(Ordering::Relaxed) {
            return Err("The microphone stream was interrupted. Check the default input device and try again.".into());
        }
        let frame = self.queue.pop(timeout)?;
        match frame {
            Some(frame) => {
                let mut echo = self
                    .echo
                    .lock()
                    .map_err(|_| "Microphone processing unavailable")?;
                if self.queue.discontinuity.swap(false, Ordering::Relaxed) {
                    echo.reset()?;
                    self.discontinuity.store(true, Ordering::Relaxed);
                }
                Ok(Some(echo.filter(frame)?))
            }
            None => Ok(None),
        }
    }
    fn take_discontinuity(&self) -> bool {
        self.discontinuity.swap(false, Ordering::Relaxed)
    }
}
fn build<T: SizedSample>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    queue: Arc<CaptureQueue>,
    failed: Arc<AtomicBool>,
) -> Result<cpal::Stream, String>
where
    f32: FromSample<T>,
{
    let channels = usize::from(config.channels);
    let rate = config.sample_rate;
    let mut phase = 0u32;
    let mut sum = 0f32;
    let mut count = 0u32;
    let mut packet = Vec::with_capacity(512);
    let errors = failed.clone();
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                for frame in data.chunks_exact(channels) {
                    let mono =
                        frame.iter().map(|s| f32::from_sample(*s)).sum::<f32>() / channels as f32;
                    sum += if mono.is_finite() {
                        mono.clamp(-1., 1.)
                    } else {
                        0.
                    };
                    count += 1;
                    phase += 16_000;
                    if phase >= rate {
                        phase -= rate;
                        packet.push(sum / count as f32);
                        sum = 0.;
                        count = 0;
                        if packet.len() == 512 {
                            let ready = std::mem::replace(&mut packet, Vec::with_capacity(512));
                            queue.push(ready);
                        }
                    }
                }
            },
            move |_| {
                errors.store(true, Ordering::Relaxed);
            },
            None,
        )
        .map_err(|_| {
            "Cannot open the microphone. Check system permissions and the default input device."
                .into()
        })
}
impl VoiceInput for NativeVoiceInput {
    fn open(&self) -> Result<Box<dyn InputFrames>, String> {
        let device = cpal::default_host()
            .default_input_device()
            .ok_or("No default microphone found")?;
        let supported = device
            .default_input_config()
            .map_err(|_| "Cannot read microphone format")?;
        let config: cpal::StreamConfig = supported.into();
        if config.channels == 0
            || config.channels > 8
            || !(16_000..=192_000).contains(&config.sample_rate)
        {
            return Err("Voice mode needs a microphone format between 16 and 192 kHz.".into());
        }
        let queue = Arc::new(CaptureQueue::default());
        let echo = super::echo::CaptureEcho::new(self.0.clone())?;
        let failed = Arc::new(AtomicBool::new(false));
        let stream = match supported.sample_format() {
            SampleFormat::F32 => build::<f32>(&device, config, queue.clone(), failed.clone())?,
            SampleFormat::I16 => build::<i16>(&device, config, queue.clone(), failed.clone())?,
            SampleFormat::U16 => build::<u16>(&device, config, queue.clone(), failed.clone())?,
            SampleFormat::I32 => build::<i32>(&device, config, queue.clone(), failed.clone())?,
            _ => return Err("Unsupported microphone sample format".into()),
        };
        stream
            .play()
            .map_err(|_| "Cannot start voice mode microphone")?;
        Ok(Box::new(Frames {
            echo: Mutex::new(echo),
            discontinuity: AtomicBool::new(false),
            _stream: stream,
            queue,
            failed,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stalled_consumer_recovers_with_recent_bounded_audio() {
        let queue = CaptureQueue::default();
        for value in 0..100 {
            queue.push(vec![value as f32; 512]);
        }
        assert_eq!(queue.frames.lock().unwrap().len(), CAPTURE_PACKETS);
        assert!(queue.discontinuity.swap(false, Ordering::Relaxed));
        assert_eq!(queue.pop(Duration::ZERO).unwrap().unwrap()[0], 92.);
        for _ in 0..7 {
            queue.pop(Duration::ZERO).unwrap();
        }
        queue.push(vec![100.; 512]);
        assert_eq!(queue.pop(Duration::ZERO).unwrap().unwrap()[0], 100.);
        assert!(!queue.discontinuity.load(Ordering::Relaxed));
    }
}
