import { Headphones, Square, X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { ActivityOrb } from '@/components/feedback/ActivityOrb';
import type { VoiceModeController } from './useVoiceMode';
export function VoiceModeButton({
  mode,
  disabled,
}: {
  mode: VoiceModeController;
  disabled: boolean;
}) {
  return (
    <div className="voice-mode-entry">
      <Button
        variant="outline"
        disabled={disabled || mode.starting}
        onClick={() => void mode.start()}
      >
        <Headphones className="size-4" aria-hidden="true" />
        {mode.starting ? 'Starting voice…' : 'Voice mode'}
      </Button>
      {mode.error && (
        <div role="alert" className="voice-mode-error">
          <span>{mode.error}</span>
          <Button
            variant="ghost"
            size="icon"
            aria-label="Dismiss voice mode error"
            onClick={() => mode.setError('')}
          >
            <X className="size-4" aria-hidden="true" />
          </Button>
        </div>
      )}
    </div>
  );
}
export function VoiceConversation({ mode }: { mode: VoiceModeController }) {
  const waiting = mode.phase === 'waiting',
    listening = mode.phase === 'listening',
    speaking = mode.phase === 'speaking';
  const label = waiting
    ? 'Say “Hey Jarvis”'
    : listening
      ? 'I’m listening'
      : speaking
        ? 'JARVIS is speaking'
        : mode.phase === 'transcribing'
          ? 'Understanding your request…'
          : mode.phase === 'starting'
            ? 'Opening local voice…'
            : 'Thinking it through…';
  return (
    <section className="voice-conversation" aria-label="Voice conversation">
      <div className="eyebrow">LOCAL VOICE CONVERSATION</div>
      <div className="conversation-orb">
        <ActivityOrb
          size={64}
          state={
            waiting ? 'breathing' : listening ? 'listening' : speaking ? 'composing' : 'solving'
          }
        />
      </div>
      <h2 role="status" aria-live="polite">
        {label}
      </h2>
      <p>
        {waiting
          ? 'Say the wake phrase, then ask your question.'
          : listening
            ? 'Speak naturally. Pause when you’re finished.'
            : speaking
              ? 'The microphone is paused while JARVIS responds.'
              : 'Your request is being processed on this device.'}
      </p>
      <div className="voice-input-meter">
        <div
          role="meter"
          aria-label="Microphone input level"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(mode.inputLevel * 100)}
        >
          <span style={{ width: `${mode.inputLevel * 100}%` }} />
        </div>
        <small>
          {waiting || listening
            ? mode.inputLevel > 0.25
              ? 'Audio detected · microphone is receiving sound'
              : 'Speak to check your microphone'
            : 'Microphone paused'}
        </small>
      </div>
      <div className="voice-turn-preview">
        {mode.lastUser && (
          <div>
            <span>You</span>
            <p>{mode.lastUser}</p>
          </div>
        )}
        {mode.lastReply && (
          <div>
            <span>JARVIS</span>
            <p>
              {mode.lastReply.length > 400 ? `${mode.lastReply.slice(0, 400)}…` : mode.lastReply}
            </p>
          </div>
        )}
      </div>
      <Button
        autoFocus
        aria-keyshortcuts="Escape"
        className="voice-end-button"
        variant="outline"
        onClick={() => void mode.stop()}
      >
        <Square className="size-4" aria-hidden="true" />
        Stop voice mode
      </Button>
      <small>
        Microphone active while listening · Spoken requests send automatically and stay in local
        chat history.
      </small>
      <small>
        Press Esc to stop. Listening ends when you switch chats, close, or minimize the app.
      </small>
    </section>
  );
}
