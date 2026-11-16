import { useCallback, useEffect, useRef, useState } from 'react';
import type { VoiceGateway, VoiceStatus } from '@/domain/voice';
import type { Message } from '@/domain/models';
import { errorText } from '@/lib/errors';

export function useVoice(
  api: VoiceGateway,
  desktop: boolean,
  conversationId: string | null,
  onTranscript: (text: string) => void,
) {
  const [status, setStatus] = useState<VoiceStatus | null>(null);
  const [pending, setPending] = useState<'start' | 'finish' | 'download' | 'speak' | null>(null);
  const [error, setError] = useState('');
  const [setupOpen, setSetupOpen] = useState(false);
  const operation = useRef(false);
  const session = useRef(0);
  const capture = useRef<{ conversationId: string | null } | null>(null);
  const target = useRef(conversationId);
  target.current = conversationId;
  const receive = useRef(onTranscript);
  receive.current = onTranscript;
  const alive = useRef(true);
  const refresh = useCallback(async () => {
    const value = await api.status();
    if (alive.current) setStatus(value);
    return value;
  }, [api]);

  useEffect(() => {
    if (!desktop) return;
    alive.current = true;
    void refresh()
      .then(async (value) => {
        if (value.stage === 'recording' || value.stage === 'transcribing') {
          await api.cancel();
          if (alive.current) await refresh();
        }
      })
      .catch((e) => {
        if (alive.current) setError(errorText(e));
      });
    return () => {
      alive.current = false;
      session.current++;
      void api.cancel().catch(() => {});
      void api.stop().catch(() => {});
    };
  }, [api, desktop, refresh]);
  useEffect(() => {
    if (
      !desktop ||
      (!pending &&
        status?.stage !== 'recording' &&
        status?.stage !== 'transcribing' &&
        status?.stage !== 'downloading' &&
        !status?.speaking)
    )
      return;
    let fetching = false;
    const timer = window.setInterval(() => {
      if (fetching) return;
      fetching = true;
      void refresh()
        .catch((e) => {
          if (alive.current) setError(errorText(e));
        })
        .finally(() => {
          fetching = false;
        });
    }, 500);
    return () => window.clearInterval(timer);
  }, [desktop, pending, status?.stage, status?.speaking, refresh]);
  useEffect(() => {
    if (capture.current && capture.current.conversationId !== conversationId) {
      session.current++;
      capture.current = null;
      void api
        .cancel()
        .then(refresh)
        .catch(() => {});
    }
  }, [api, conversationId, refresh]);

  async function start() {
    if (!desktop || operation.current) return;
    if (!status?.model.ready) {
      setSetupOpen(true);
      return;
    }
    operation.current = true;
    setPending('start');
    setError('');
    const token = ++session.current;
    capture.current = { conversationId };
    try {
      await api.start();
      if (token !== session.current) {
        await api.cancel();
      }
      await refresh();
    } catch (e) {
      capture.current = null;
      if (alive.current) setError(errorText(e));
    } finally {
      operation.current = false;
      if (alive.current) setPending(null);
    }
  }
  async function finish() {
    if (!desktop || operation.current) return;
    operation.current = true;
    setPending('finish');
    setError('');
    const token = session.current;
    try {
      const transcript = await api.finish();
      if (token === session.current && capture.current?.conversationId === target.current)
        receive.current(transcript);
    } catch (e) {
      if (token === session.current && alive.current) setError(errorText(e));
    } finally {
      capture.current = null;
      operation.current = false;
      if (alive.current) {
        setPending(null);
        await refresh().catch(() => {});
      }
    }
  }
  async function download() {
    if (!desktop || operation.current) return;
    operation.current = true;
    setPending('download');
    setError('');
    try {
      await api.download();
      await refresh();
    } catch (e) {
      if (alive.current && !/cancelled/i.test(errorText(e))) setError(errorText(e));
    } finally {
      operation.current = false;
      if (alive.current) {
        setPending(null);
        await refresh().catch(() => {});
      }
    }
  }
  async function cancel() {
    session.current++;
    capture.current = null;
    try {
      await api.cancel();
      await refresh();
    } catch (e) {
      if (alive.current) setError(errorText(e));
    }
  }
  async function speak(message: Message) {
    if (!desktop || operation.current) return;
    operation.current = true;
    setPending('speak');
    setError('');
    try {
      await api.speak(message.conversation_id, message.id);
      await refresh();
    } catch (e) {
      if (alive.current) setError(errorText(e));
    } finally {
      operation.current = false;
      if (alive.current) setPending(null);
    }
  }
  async function stop() {
    try {
      await api.stop();
      await refresh();
    } catch (e) {
      setError(errorText(e));
    }
  }
  useEffect(() => {
    if (status?.stage === 'recording' && status.recording.seconds >= 30 && !operation.current)
      void finish();
  }, [status]);

  const stage = status?.stage ?? 'idle';
  const dictating =
    stage === 'recording' ||
    stage === 'transcribing' ||
    pending === 'start' ||
    pending === 'finish';
  return {
    available: desktop,
    status,
    stage,
    pending,
    dictating,
    error,
    setError,
    setupOpen,
    setSetupOpen,
    start,
    finish,
    download,
    cancel,
    speak,
    stop,
  };
}
export type VoiceController = ReturnType<typeof useVoice>;
