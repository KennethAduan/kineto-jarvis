import { useEffect, useState } from 'react';
import { Download, Play, Square, Volume2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Slider } from '@/components/ui/slider';
import { ActivityOrb } from '@/components/feedback/ActivityOrb';
import {
  speechVoices,
  type SpeechPreferences,
  type SpeechVoice,
  type SpeechGateway,
} from '@/domain/speech';
import { useSpeech } from './useSpeech';

export function SpeechSettings({
  api,
  desktop,
  dictating,
}: {
  api: SpeechGateway;
  desktop: boolean;
  dictating: boolean;
}) {
  const speech = useSpeech(api, desktop);
  const [preferences, setPreferences] = useState<SpeechPreferences>({
    voice: 'supertonic_m2',
    speed: 1.15,
    volume: 1,
  });
  const voice = speech.status?.preferences.voice;
  const speed = speech.status?.preferences.speed;
  const volume = speech.status?.preferences.volume;
  useEffect(() => {
    if (voice && speed !== undefined) setPreferences({ voice, speed, volume: volume ?? 1 });
  }, [voice, speed, volume]);
  const phase = speech.status?.phase ?? 'idle';
  const downloading = phase === 'downloading' || speech.pending === 'download';
  const playing = phase === 'playing' || phase === 'preparing';
  const supertonic = preferences.voice.startsWith('supertonic');
  const engine = supertonic ? 'Supertonic 3' : 'Kokoro';
  const bytes = supertonic ? 145311845 : 148782633;
  const ready =
    speech.status?.packs?.[supertonic ? 'supertonic' : 'kokoro'].ready ??
    speech.status?.model.ready ??
    false;
  const system = preferences.voice === 'system';
  const disabled = !desktop || !speech.status || !!speech.pending || phase !== 'idle' || dictating;
  const changed =
    preferences.voice !== voice || preferences.speed !== speed || preferences.volume !== volume;
  return (
    <section className="speech-settings" aria-labelledby="speech-heading">
      <div className="speech-heading">
        <Volume2 className="size-4" aria-hidden="true" />
        <h3 id="speech-heading">Spoken replies</h3>
      </div>
      <p>
        Supertonic 3 is the new conversational voice. Kokoro British voices remain available. Speech
        runs on this device.
      </p>
      <div className="speech-field">
        <label htmlFor="speech-voice">Voice</label>
        <Select
          value={preferences.voice}
          onValueChange={(value) => setPreferences({ ...preferences, voice: value as SpeechVoice })}
          disabled={disabled}
        >
          <SelectTrigger id="speech-voice" className="speech-select">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {speechVoices.map(({ id, label }) => (
              <SelectItem key={id} value={id}>
                {label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      <div className="speech-field">
        <div className="speech-speed-label">
          <label id="speech-speed-label" htmlFor="speech-speed">
            Speaking speed
          </label>
          <output htmlFor="speech-speed">{preferences.speed.toFixed(2)}×</output>
        </div>
        <Slider
          id="speech-speed"
          aria-labelledby="speech-speed-label"
          value={[preferences.speed]}
          onValueChange={([speed]) => setPreferences({ ...preferences, speed })}
          min={0.75}
          max={1.5}
          step={0.05}
          disabled={disabled || system}
        />
        <small>
          {system
            ? 'The system voice uses its default speed.'
            : 'Try 1.15× for a natural conversation, or increase up to 1.50×.'}
        </small>
      </div>
      <div className="speech-field">
        <div className="speech-speed-label">
          <label id="speech-volume-label" htmlFor="speech-volume">
            Output volume
          </label>
          <output htmlFor="speech-volume">{Math.round(preferences.volume * 100)}%</output>
        </div>
        <Slider
          id="speech-volume"
          aria-labelledby="speech-volume-label"
          value={[preferences.volume]}
          min={0}
          max={1.5}
          step={0.05}
          onValueChange={([volume]) => setPreferences({ ...preferences, volume })}
          disabled={disabled || system}
        />
        <small>
          {system
            ? 'Adjust the system voice in your operating system sound settings.'
            : 'Quiet speech is normalized automatically. Peak protection prevents digital clipping.'}
        </small>
      </div>
      {!system && !ready && (
        <p className="speech-setup-hint">
          Download the {engine} voice pack once to enable previews and spoken replies. About{' '}
          {Math.round(bytes / 1000000)} MB, with verified checksums.
        </p>
      )}
      <div className="speech-actions">
        <Button
          variant="outline"
          disabled={disabled || (!system && !ready)}
          onClick={() => void speech.preview(preferences)}
        >
          <Play className="size-4" aria-hidden="true" /> Preview voice
        </Button>
        <Button
          disabled={disabled || (!changed && !speech.error)}
          onClick={() => void speech.save(preferences)}
        >
          {speech.pending === 'save' ? 'Saving…' : 'Save voice'}
        </Button>
      </div>
      {speech.saved && !changed && (
        <p className="voice-ready" role="status">
          Voice saved for future replies.
        </p>
      )}
      {(!system || downloading) && (
        <div className="speech-download">
          <Button
            variant="outline"
            disabled={disabled}
            onClick={() => void speech.download(preferences.voice)}
          >
            <Download className="size-4" aria-hidden="true" />
            {ready
              ? `Reinstall ${engine} voice pack`
              : `Download ${engine} · ${Math.round(bytes / 1000000)} MB`}
          </Button>
          {ready && <small className="voice-ready">{engine} voice pack ready.</small>}
        </div>
      )}
      {(downloading || playing) && (
        <div className="speech-progress" role="status">
          <ActivityOrb
            state={downloading ? 'connecting' : phase === 'preparing' ? 'composing' : 'breathing'}
          />
          <span>
            {downloading
              ? `Downloading ${engine} · ${Math.min(100, Math.round(((speech.status?.downloaded_bytes ?? 0) / (speech.status?.model.bytes || 148782633)) * 100))}%`
              : phase === 'preparing'
                ? 'Preparing local voice…'
                : 'Playing voice'}
          </span>
          <Button variant="ghost" onClick={() => void speech.stop()}>
            <Square className="size-4" aria-hidden="true" />
            {downloading ? 'Cancel download' : 'Stop voice'}
          </Button>
        </div>
      )}
      {downloading && (
        <progress
          aria-label={`${engine} download progress`}
          value={speech.status?.downloaded_bytes ?? 0}
          max={speech.status?.model.bytes ?? 148782633}
        />
      )}
      {speech.error && (
        <p className="voice-error" role="alert">
          {speech.error}
        </p>
      )}
      <small>
        Preview uses the current selection. Save it to apply to completed replies. Code blocks are
        skipped; read-aloud is manual and capped at 8,000 characters.
      </small>
    </section>
  );
}
