import { Headphones, Square, X, Mic, Cpu, Volume2, ShieldCheck } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { ActivityOrb } from '@/components/feedback/ActivityOrb';
import { AudioInputDisplay } from './AudioInputDisplay';
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
    <section className="voice-conversation audio-console" aria-label="Voice conversation">
      <div className="audio-console-heading">
        <span className="eyebrow">JARVIS / VOICE WORKSPACE</span>
        <span>
          <ShieldCheck size={13} /> LOCAL SESSION
        </span>
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
      <div className="jarvis-audio-deck">
        <div className="audio-deck-topline">
          <span>JARVIS</span>
          <span className={`audio-session-light ${waiting || listening ? 'live' : ''}`}>
            {waiting || listening ? 'LISTENING' : speaking ? 'VOICE OUT' : 'PROCESSING'}
          </span>
        </div>
        <div className="audio-identity">
          <div className="conversation-orb">
            <ActivityOrb
              size={64}
              state={
                waiting ? 'breathing' : listening ? 'listening' : speaking ? 'composing' : 'solving'
              }
            />
          </div>
          <div>
            <span className="audio-engine-label">YOUR LOCAL ASSISTANT</span>
            <h2 role="status" aria-live="polite">
              {label}
            </h2>
            <p>
              {waiting
                ? 'Say the wake phrase, then ask your question.'
                : listening
                  ? 'Speak naturally. Pause when you’re finished.'
                  : speaking
                    ? 'Speak to interrupt JARVIS. Echo reduction runs on this device.'
                    : 'Your request is being processed on this device.'}
            </p>
          </div>
        </div>
        <AudioInputDisplay
          level={mode.inputLevel}
          sequence={mode.inputSequence}
          active={mode.micActive}
        />
        {mode.audioNotice && (
          <p className="audio-recovery-notice" role="status">
            {mode.audioNotice}
          </p>
        )}
        <div className="audio-signal-chain" aria-label="Conversation stages">
          <div data-active={mode.micActive}>
            <Mic size={15} />
            <span>01 / HEAR</span>
            <small>Local microphone</small>
          </div>
          <div data-active={!waiting && !listening && !speaking}>
            <Cpu size={15} />
            <span>02 / THINK</span>
            <small>LM Studio</small>
          </div>
          <div data-active={speaking}>
            <Volume2 size={15} />
            <span>03 / SPEAK</span>
            <small>Local voice</small>
          </div>
        </div>
        <div className="audio-deck-footer">
          <span>ON-DEVICE AUDIO ENGINE</span>
          <span>NO CLOUD ROUTING</span>
        </div>
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

      <small>
        Spoken requests send automatically and stay in local chat history. Press Esc to stop.
      </small>
      <small>Listening ends when you switch chats, close, or minimize the app.</small>
    </section>
  );
}
