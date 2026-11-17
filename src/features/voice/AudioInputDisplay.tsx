import { useEffect, useMemo, useRef } from 'react';
import {
  LevelMeter,
  LevelMeterValue,
  LevelMeterChannels,
  LevelMeterChannel,
  LevelMeterTrack,
  LevelMeterBar,
} from '@/components/ui/level-meter';
import { LiveWaveform } from '@/components/ui/live-waveform';
import { createFrameEmitter } from '@/lib/frame-source';
import type { VisualFrame } from '@/lib/types';

/** Display the measured RMS envelope from Rust. Never opens a browser microphone. */
export function AudioInputDisplay({
  level,
  sequence,
  active,
}: {
  level: number;
  sequence: number;
  active: boolean;
}) {
  const source = useMemo(() => createFrameEmitter<VisualFrame>(), []);
  const history = useRef({ values: new Float32Array(64), count: 0 });
  const db = active ? Math.max(-60, Math.min(0, level * 60 - 60)) : -60;
  useEffect(() => {
    const ring = history.current;
    if (!active) {
      ring.values.fill(0);
      ring.count = 0;
      return;
    }
    ring.values[ring.count % 64] = level;
    ring.count++;
    source.emit({
      bands: new Float32Array(0),
      history: ring.values,
      historyStart: ring.count > 64 ? ring.count % 64 : 0,
      historyLength: Math.min(64, ring.count),
      historyUpdatedAt: performance.now(),
      historyIntervalMs: 100,
      peakDb: db,
    });
  }, [active, db, level, sequence, source]);
  return (
    <div className="audio-input-display">
      <div className="audio-channel-heading">
        <span>01 / MICROPHONE</span>
        <span>{active ? 'INPUT OPEN' : 'INPUT PAUSED'}</span>
      </div>
      <LiveWaveform
        source={source}
        mode="scrolling"
        variant="bars"
        active={active}
        className="audio-live-waveform"
        aria-label="Microphone activity history"
      />
      <div className="audio-meter-heading">
        <span>INPUT LEVEL · RMS</span>
        <span>dBFS</span>
      </div>
      <LevelMeter
        aria-label="Microphone input level"
        peakDb={db}
        rmsDb={db}
        ballistics="instant"
        variant="segmented"
        segments={36}
        className="jarvis-level-meter"
      >
        <LevelMeterChannels>
          <LevelMeterChannel>
            <LevelMeterTrack>
              <LevelMeterBar measure="rms" />
            </LevelMeterTrack>
          </LevelMeterChannel>
        </LevelMeterChannels>
        <LevelMeterValue />
      </LevelMeter>
      <small>
        {active
          ? level > 0.25
            ? 'Audio detected · microphone is receiving sound'
            : 'Speak to check your microphone'
          : 'Microphone paused'}
      </small>
    </div>
  );
}
