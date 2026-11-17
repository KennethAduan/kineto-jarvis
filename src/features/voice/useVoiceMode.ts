import { useCallback, useEffect, useRef, useState } from 'react';
import type { VoiceModeGateway, VoiceModeEvent } from '@/domain/voice-mode';
import type { VoiceGateway } from '@/domain/voice';
import type { AssistantController } from '@/features/assistant/useAssistant';
import { errorText } from '@/lib/errors';

export function useVoiceMode(
  api: VoiceModeGateway,
  voice: VoiceGateway,
  assistant: AssistantController,
  onSetup: () => void,
) {
  const [enabled, setEnabled] = useState(false);
  const [phase, setPhase] = useState('off');
  const [error, setError] = useState('');
  const [lastUser, setLastUser] = useState('');
  const [lastReply, setLastReply] = useState('');
  const [starting, setStarting] = useState(false);
  const epoch = useRef(0),
    session = useRef<string | null>(null),
    conversation = useRef<string | null>(null),
    operation = useRef(false),
    alive = useRef(true);
  const latest = useRef({ assistant, onSetup });
  latest.current = { assistant, onSetup };
  const stop = useCallback(async () => {
    epoch.current++;
    session.current = null;
    operation.current = false;
    if (alive.current) {
      setEnabled(false);
      setStarting(false);
      setPhase('off');
    }
    await api.stop().catch((error) => {
      if (alive.current) setError(errorText(error));
    });
  }, [api]);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      epoch.current++;
      session.current = null;
      void api.stop().catch(() => {});
    };
  }, [api]);
  useEffect(() => {
    if (
      session.current &&
      (assistant.selected !== conversation.current || assistant.page !== 'chat')
    )
      void stop();
  }, [assistant.selected, assistant.page, enabled, stop]);
  useEffect(() => {
    if (!enabled) return;
    let pending = false;
    const timer = window.setInterval(() => {
      if (pending || !session.current) return;
      pending = true;
      const id = session.current;
      void api
        .heartbeat(id)
        .catch(() => {
          if (session.current === id) return stop();
        })
        .finally(() => {
          pending = false;
        });
    }, 1000);
    const hidden = () => {
      if (document.hidden) void stop();
    };
    document.addEventListener('visibilitychange', hidden);
    const escape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') void stop();
    };
    document.addEventListener('keydown', escape);
    return () => {
      window.clearInterval(timer);
      document.removeEventListener('visibilitychange', hidden);
      document.removeEventListener('keydown', escape);
    };
  }, [api, enabled, stop]);
  async function answer(text: string, token: number) {
    if (operation.current || !session.current || token !== epoch.current) return;
    if (
      latest.current.assistant.selected !== conversation.current ||
      latest.current.assistant.page !== 'chat'
    ) {
      await stop();
      return;
    }
    operation.current = true;
    setLastUser(text);
    setLastReply('');
    setPhase('thinking');
    try {
      const message = await latest.current.assistant.send(text);
      if (token !== epoch.current) return;
      if (!message)
        throw new Error('JARVIS could not complete the reply. Check the local model connection.');
      setLastReply(message.content);
      setPhase('speaking');
      await voice.speak(message.conversation_id, message.id);
      while (token === epoch.current) {
        const status = await voice.status();
        if (status.playback_error) throw new Error(status.playback_error);
        if (!status.speaking) break;
        await new Promise((resolve) => window.setTimeout(resolve, 200));
      }
      if (token !== epoch.current || !session.current) return;
      await new Promise((resolve) => window.setTimeout(resolve, 200));
      if (token === epoch.current && session.current) await api.resume(session.current);
    } catch (error) {
      if (token === epoch.current) {
        setError(errorText(error));
        await stop();
      }
    } finally {
      if (token === epoch.current) operation.current = false;
    }
  }
  async function start() {
    if (starting || session.current) return;
    setStarting(true);
    setError('');
    setLastUser('');
    setLastReply('');
    const token = ++epoch.current;
    try {
      const status = await api.status();
      if (!status.model.ready) {
        latest.current.onSetup();
        throw new Error('Download the Hey Jarvis listening pack in Voice settings first.');
      }
      const id = await latest.current.assistant.ensureConversation();
      if (token !== epoch.current) return;
      conversation.current = id;
      const receive = (event: VoiceModeEvent) => {
        if (token !== epoch.current || !alive.current) return;
        if (event.kind === 'phase') {
          if (event.phase !== 'paused') setPhase(event.phase);
        } else if (event.kind === 'transcript') void answer(event.text, token);
        else if (event.kind === 'error') {
          setError(event.message);
          void stop();
        } else if (event.kind === 'stopped') {
          epoch.current++;
          session.current = null;
          setEnabled(false);
          setPhase('off');
        }
      };
      const sessionId = await api.start(receive);
      if (token !== epoch.current) {
        await api.stop();
        return;
      }
      session.current = sessionId;
      setEnabled(true);
      await api.heartbeat(sessionId);
    } catch (error) {
      if (token === epoch.current && alive.current) {
        setError(errorText(error));
        if (/download/i.test(errorText(error))) latest.current.onSetup();
      }
    } finally {
      if (alive.current) setStarting(false);
    }
  }
  return { enabled, starting, phase, error, setError, lastUser, lastReply, start, stop };
}
export type VoiceModeController = ReturnType<typeof useVoiceMode>;
