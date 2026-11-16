use crate::domain::voice::{AudioRecorder, CapturedAudio, RecordingInfo, MAX_RECORDING_SECONDS};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SampleFormat, SizedSample,
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

struct Capture {
    samples: Vec<f32>,
    error: Option<String>,
}
struct Recording {
    stream: Option<cpal::Stream>,
    capture: Arc<Mutex<Capture>>,
    started: Instant,
    sample_rate: u32,
    id: uuid::Uuid,
}
#[derive(Default)]
pub struct Microphone {
    recording: Arc<Mutex<Option<Recording>>>,
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    capture: Arc<Mutex<Capture>>,
) -> Result<cpal::Stream, String>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = usize::from(config.channels);
    let limit = config.sample_rate as usize * MAX_RECORDING_SECONDS as usize;
    let errors = capture.clone();
    device
        .build_input_stream(
            *config,
            move |data: &[T], _| {
                if let Ok(mut target) = capture.try_lock() {
                    let remaining = limit.saturating_sub(target.samples.len());
                    for frame in data.chunks_exact(channels).take(remaining) {
                        let mono = frame.iter().map(|sample| f32::from_sample(*sample)).sum::<f32>()
                            / channels as f32;
                        target.samples.push(if mono.is_finite() { mono.clamp(-1., 1.) } else { 0. });
                    }
                }
            },
            move |_error| {
                if let Ok(mut target) = errors.lock() {
                    target.error = Some("Microphone stopped unexpectedly. Check the input device and try again.".into());
                }
            },
            None,
        )
        .map_err(|_| "Cannot open the microphone. Check your system microphone permissions and default input device.".into())
}
impl AudioRecorder for Microphone {
    fn start(&self) -> Result<(), String> {
        let mut slot = self
            .recording
            .lock()
            .map_err(|_| "Microphone state unavailable")?;
        if slot.is_some() {
            return Err("A recording is already active".into());
        }
        let device = cpal::default_host()
            .default_input_device()
            .ok_or("No microphone found. Choose a default input device in system settings.")?;
        let supported = device
            .default_input_config()
            .map_err(|_| "Could not read the microphone format")?;
        let config: cpal::StreamConfig = supported.into();
        if config.channels == 0
            || config.channels > 8
            || !(8_000..=192_000).contains(&config.sample_rate)
        {
            return Err("The default microphone format is not supported.".into());
        }
        let capture = Arc::new(Mutex::new(Capture {
            samples: Vec::with_capacity(
                config.sample_rate as usize * MAX_RECORDING_SECONDS as usize,
            ),
            error: None,
        }));
        let stream=match supported.sample_format() {
            SampleFormat::F32=>build_stream::<f32>(&device,&config,capture.clone())?,
            SampleFormat::I16=>build_stream::<i16>(&device,&config,capture.clone())?,
            SampleFormat::U16=>build_stream::<u16>(&device,&config,capture.clone())?,
            SampleFormat::I32=>build_stream::<i32>(&device,&config,capture.clone())?,
            _=>return Err("This microphone sample format is not supported. Try a standard 16-bit or float input format.".into()),
        };
        stream
            .play()
            .map_err(|_| "Could not start the microphone. Check system privacy settings.")?;
        let id = uuid::Uuid::new_v4();
        *slot = Some(Recording {
            stream: Some(stream),
            capture,
            started: Instant::now(),
            sample_rate: config.sample_rate,
            id,
        });
        let weak = Arc::downgrade(&self.recording);
        // A native deadline closes the microphone even if the WebView is hidden or suspended.
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(u64::from(MAX_RECORDING_SECONDS)));
            if let Some(recording) = weak.upgrade() {
                if let Ok(mut slot) = recording.lock() {
                    if let Some(current) = slot.as_mut() {
                        if current.id == id {
                            current.stream.take();
                        }
                    }
                }
            }
        });
        Ok(())
    }
    fn finish(&self) -> Result<CapturedAudio, String> {
        let mut recording = self
            .recording
            .lock()
            .map_err(|_| "Microphone state unavailable")?
            .take()
            .ok_or("No recording is active")?;
        recording.stream.take();
        let mut capture = recording
            .capture
            .lock()
            .map_err(|_| "Microphone buffer unavailable")?;
        if let Some(error) = capture.error.take() {
            return Err(error);
        }
        Ok(CapturedAudio {
            samples: std::mem::take(&mut capture.samples),
            sample_rate: recording.sample_rate,
        })
    }
    fn cancel(&self) -> Result<(), String> {
        self.recording
            .lock()
            .map_err(|_| "Microphone state unavailable")?
            .take();
        Ok(())
    }
    fn info(&self) -> RecordingInfo {
        self.recording
            .lock()
            .ok()
            .and_then(|r| {
                r.as_ref().map(|r| RecordingInfo {
                    active: r.stream.is_some(),
                    seconds: r
                        .started
                        .elapsed()
                        .as_secs_f32()
                        .min(MAX_RECORDING_SECONDS as f32),
                })
            })
            .unwrap_or(RecordingInfo {
                active: false,
                seconds: 0.,
            })
    }
}
