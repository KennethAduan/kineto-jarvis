use crate::domain::voice_mode::{InputFrames, VoiceInput};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SampleFormat, SizedSample,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{sync_channel, Receiver, RecvTimeoutError, SyncSender},
        Arc,
    },
    time::Duration,
};
pub struct NativeVoiceInput(pub Arc<super::echo::RenderReference>);
struct Frames {
    _stream: cpal::Stream,
    receiver: Receiver<Vec<f32>>,
    failed: Arc<AtomicBool>,
    echo: std::sync::Mutex<super::echo::CaptureEcho>,
}
impl InputFrames for Frames {
    fn next(&self, timeout: Duration) -> Result<Option<Vec<f32>>, String> {
        if self.failed.load(Ordering::Relaxed) {
            return Err("Voice audio was interrupted or processing fell behind. Check the microphone and try again.".into());
        }
        match self.receiver.recv_timeout(timeout) {
            Ok(frame) => Ok(Some(
                self.echo
                    .lock()
                    .map_err(|_| "Microphone processing unavailable")?
                    .filter(frame)?,
            )),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(_) => Err("Microphone disconnected".into()),
        }
    }
}
fn build<T: SizedSample>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    sender: SyncSender<Vec<f32>>,
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
                            if sender.try_send(ready).is_err() {
                                failed.store(true, Ordering::Relaxed);
                            }
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
        let (sender, receiver) = sync_channel(64);
        let failed = Arc::new(AtomicBool::new(false));
        let stream = match supported.sample_format() {
            SampleFormat::F32 => build::<f32>(&device, config, sender, failed.clone())?,
            SampleFormat::I16 => build::<i16>(&device, config, sender, failed.clone())?,
            SampleFormat::U16 => build::<u16>(&device, config, sender, failed.clone())?,
            SampleFormat::I32 => build::<i32>(&device, config, sender, failed.clone())?,
            _ => return Err("Unsupported microphone sample format".into()),
        };
        stream
            .play()
            .map_err(|_| "Cannot start voice mode microphone")?;
        Ok(Box::new(Frames {
            echo: std::sync::Mutex::new(super::echo::CaptureEcho::new(self.0.clone())?),
            _stream: stream,
            receiver,
            failed,
        }))
    }
}
