import { Mic, Settings2, Square, Volume2, VolumeX, X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { InputGroupButton } from '@/components/ui/input-group';
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from '@/components/ui/sheet';
import { ActivityOrb } from '@/components/feedback/ActivityOrb';
import type { VoiceController } from './useVoice';

export function VoiceMicButton({ voice, disabled }: { voice: VoiceController; disabled: boolean }) {
  const recording = voice.stage === 'recording';
  return (
    <InputGroupButton
      size="icon-sm"
      variant={recording ? 'secondary' : 'ghost'}
      className="voice-mic-button"
      disabled={
        disabled || !voice.available || !!voice.pending || (!recording && voice.stage !== 'idle')
      }
      aria-label={recording ? 'Finish dictation' : 'Record voice message'}
      aria-pressed={recording}
      title={recording ? 'Finish dictation' : 'Record voice message — up to 30 seconds'}
      onClick={() => void (recording ? voice.finish() : voice.start())}
    >
      {recording ? (
        <Square className="size-4" fill="currentColor" aria-hidden="true" />
      ) : (
        <Mic className="size-4" aria-hidden="true" />
      )}
    </InputGroupButton>
  );
}

export function VoiceActivity({ voice }: { voice: VoiceController }) {
  const active = voice.stage !== 'idle' || voice.pending === 'start' || voice.pending === 'finish';
  return (
    <>
      {voice.error && (
        <div className="voice-error" role="alert">
          <span>{voice.error}</span>
          <Button
            variant="ghost"
            size="icon"
            aria-label="Dismiss voice error"
            onClick={() => voice.setError('')}
          >
            <X className="size-4" aria-hidden="true" />
          </Button>
        </div>
      )}
      {active && (
        <div className="voice-activity" role="status">
          <ActivityOrb
            state={
              voice.stage === 'recording'
                ? 'listening'
                : voice.stage === 'downloading'
                  ? 'connecting'
                  : 'working'
            }
          />
          <div>
            <strong>
              {voice.stage === 'recording'
                ? `Listening · ${Math.floor(voice.status?.recording.seconds ?? 0)} / 30s`
                : voice.stage === 'downloading' || voice.pending === 'download'
                  ? `Downloading speech model · ${Math.min(100, Math.round(((voice.status?.downloaded_bytes ?? 0) / (voice.status?.model.bytes || 1)) * 100))}%`
                  : voice.pending === 'start'
                    ? 'Opening microphone…'
                    : 'Transcribing on this device…'}
            </strong>
            <small>
              {voice.stage === 'recording'
                ? 'Finish dictation to add the transcript to your draft.'
                : 'Audio stays local. Dictation is never sent automatically.'}
            </small>
          </div>
          {voice.stage === 'recording' && (
            <Button
              variant="outline"
              onClick={() => void voice.finish()}
              disabled={!!voice.pending}
            >
              Finish
            </Button>
          )}
          <Button variant="ghost" onClick={() => void voice.cancel()}>
            Cancel
          </Button>
        </div>
      )}
      {voice.status?.speaking && (
        <div className="voice-activity" role="status">
          <Volume2 className="size-4" aria-hidden="true" />
          <span>Reading reply aloud</span>
          <Button variant="ghost" onClick={() => void voice.stop()}>
            <VolumeX className="size-4" aria-hidden="true" />
            Stop speaking
          </Button>
        </div>
      )}
    </>
  );
}

export function VoiceSettingsButton({ voice }: { voice: VoiceController }) {
  return (
    <Button
      variant="ghost"
      size="icon"
      className="voice-settings-button"
      disabled={!voice.available}
      aria-label="Voice settings"
      title="Voice settings"
      onClick={() => voice.setSetupOpen(true)}
    >
      <Settings2 className="size-4" aria-hidden="true" />
    </Button>
  );
}

export function VoiceSetup({ voice }: { voice: VoiceController }) {
  const downloading = voice.stage === 'downloading' || voice.pending === 'download';
  const ready = voice.status?.model.ready ?? false;
  return (
    <Sheet open={voice.setupOpen} onOpenChange={voice.setSetupOpen}>
      <SheetContent className="voice-setup">
        <SheetHeader>
          <SheetTitle>Offline voice</SheetTitle>
          <SheetDescription>
            Dictate a message on your device, review the transcript, then send when you’re ready.
          </SheetDescription>
        </SheetHeader>
        <div className="voice-setup-body">
          <h3>One-time speech model</h3>
          <p>
            Whisper Base understands multiple languages and runs on your CPU. Setup downloads about
            148 MB from Hugging Face and verifies its checksum.
          </p>
          <p>
            After setup, audio never leaves your machine. Recordings stay in memory, are discarded
            after transcription, and the microphone closes after 30 seconds.
          </p>
          <Button
            onClick={() => void voice.download()}
            disabled={!voice.available || !!voice.pending || voice.stage !== 'idle'}
          >
            {ready ? 'Reinstall speech model' : 'Download speech model · 148 MB'}
          </Button>
          {downloading && (
            <>
              <progress
                aria-label="Speech model download progress"
                value={voice.status?.downloaded_bytes ?? 0}
                max={voice.status?.model.bytes ?? 147951465}
              />
              <Button variant="outline" onClick={() => void voice.cancel()}>
                Cancel download
              </Button>
            </>
          )}
          {ready && <p className="voice-ready">Offline speech model ready.</p>}
          <h3>Read-aloud</h3>
          <p>
            Use the speaker button on a completed reply. JARVIS uses your installed system voice and
            skips code blocks. Read-aloud is manual and limited to the first 8,000 characters.
          </p>
          <p>No wake word or always-on listening is enabled.</p>
          {voice.error && (
            <p className="voice-error" role="alert">
              {voice.error}
            </p>
          )}
        </div>
      </SheetContent>
    </Sheet>
  );
}
