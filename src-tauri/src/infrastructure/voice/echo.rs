use sonora::config::{EchoCanceller, HighPassFilter, NoiseSuppression};
use sonora::{AudioProcessing, Config, StreamConfig};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

/// Bounded, in-memory reference of the samples actually sent to the speakers.
#[derive(Default)]
pub struct RenderReference(Mutex<VecDeque<[f32; 160]>>);
impl RenderReference {
    fn push(&self, frame: [f32; 160]) {
        if let Ok(mut queue) = self.0.try_lock() {
            if queue.len() >= 100 {
                queue.pop_front();
            }
            queue.push_back(frame);
        }
    }
}
pub struct RenderFeed {
    reference: Arc<RenderReference>,
    rate: u32,
    phase: u32,
    sum: f32,
    count: u32,
    frame: [f32; 160],
    used: usize,
}
impl RenderFeed {
    pub fn new(reference: Arc<RenderReference>, rate: u32) -> Self {
        Self {
            reference,
            rate,
            phase: 0,
            sum: 0.,
            count: 0,
            frame: [0.; 160],
            used: 0,
        }
    }
    pub fn push(&mut self, value: f32) {
        self.sum += value;
        self.count += 1;
        self.phase += 16_000;
        if self.phase >= self.rate {
            let sample = self.sum / self.count as f32;
            self.sum = 0.;
            self.count = 0;
            while self.phase >= self.rate {
                self.phase -= self.rate;
                self.frame[self.used] = sample;
                self.used += 1;
                if self.used == 160 {
                    self.reference.push(self.frame);
                    self.used = 0;
                }
            }
        }
    }
}
pub struct CaptureEcho {
    apm: AudioProcessing,
    reference: Arc<RenderReference>,
    pending: VecDeque<f32>,
}
impl CaptureEcho {
    pub fn new(reference: Arc<RenderReference>) -> Result<Self, String> {
        let apm = std::panic::catch_unwind(|| {
            AudioProcessing::builder()
                .config(Config {
                    echo_canceller: Some(EchoCanceller::default()),
                    noise_suppression: Some(NoiseSuppression::default()),
                    high_pass_filter: Some(HighPassFilter::default()),
                    ..Default::default()
                })
                .capture_config(StreamConfig::new(16_000, 1))
                .render_config(StreamConfig::new(16_000, 1))
                .build()
        })
        .map_err(|_| "Could not initialize local echo cancellation")?;
        reference
            .0
            .lock()
            .map_err(|_| "Audio reference unavailable")?
            .clear();
        Ok(Self {
            apm,
            reference,
            pending: VecDeque::new(),
        })
    }
    pub fn filter(&mut self, samples: Vec<f32>) -> Result<Vec<f32>, String> {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let references: Vec<_> = self
                .reference
                .0
                .lock()
                .map_err(|_| "Audio reference unavailable")?
                .drain(..)
                .collect();
            let mut scratch = [0.; 160];
            for frame in references {
                self.apm
                    .process_render_f32(&[&frame], &mut [&mut scratch])
                    .map_err(|_| "Echo reference processing failed")?;
            }
            self.pending.extend(samples);
            let mut output = Vec::with_capacity(self.pending.len());
            while self.pending.len() >= 160 {
                let mut frame = [0.; 160];
                for sample in &mut frame {
                    *sample = self.pending.pop_front().unwrap_or_default();
                }
                self.apm
                    .set_stream_delay_ms(50)
                    .map_err(|_| "Echo delay configuration failed")?;
                self.apm
                    .process_capture_f32(&[&frame], &mut [&mut scratch])
                    .map_err(|_| "Microphone echo processing failed")?;
                output.extend(scratch.into_iter().map(|s| {
                    if s.is_finite() {
                        s.clamp(-1., 1.)
                    } else {
                        0.
                    }
                }));
            }
            Ok(output)
        }))
        .unwrap_or_else(|_| Err("Local echo cancellation stopped. Restart voice mode.".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reduces_a_delayed_synthetic_speaker_echo() {
        let reference = Arc::new(RenderReference::default());
        let mut capture = CaptureEcho::new(reference.clone()).unwrap();
        let mut feed = RenderFeed::new(reference, 16_000);
        let mut delay = VecDeque::from(vec![[0.; 160]; 5]);
        let (mut input_energy, mut output_energy) = (0_f64, 0_f64);
        for block in 0..600 {
            let mut render = [0.; 160];
            for (i, s) in render.iter_mut().enumerate() {
                let t = (block * 160 + i) as f32 / 16_000.;
                *s = 0.1 * (t * 817.).sin() * (0.7 + 0.3 * (t * 19.).sin());
                feed.push(*s);
            }
            delay.push_back(render);
            let mic: Vec<f32> = delay.pop_front().unwrap().iter().map(|s| s * 0.6).collect();
            let clean = capture.filter(mic.clone()).unwrap();
            if block > 400 {
                input_energy += mic.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>();
                output_energy += clean.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>();
            }
        }
        assert!(
            output_energy < input_energy * 0.5,
            "echo output/input energy = {}",
            output_energy / input_energy
        );
    }
    #[test]
    fn echo_reference_and_capture_are_bounded_and_never_emit_invalid_samples() {
        let reference = Arc::new(RenderReference::default());
        let mut capture = CaptureEcho::new(reference.clone()).unwrap();
        let mut feed = RenderFeed::new(reference.clone(), 48_000);
        for _ in 0..48_000 * 2 {
            feed.push(0.1);
        }
        assert!(reference.0.lock().unwrap().len() <= 100);
        for _ in 0..20 {
            let clean = capture.filter(vec![0.05; 512]).unwrap();
            assert!(clean.len() <= 640);
            assert!(clean.iter().all(|s| s.is_finite() && s.abs() <= 1.));
        }
    }
}
