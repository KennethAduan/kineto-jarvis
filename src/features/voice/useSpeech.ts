import { useCallback, useEffect, useRef, useState } from 'react';
import type { SpeechGateway, SpeechPreferences, SpeechStatus, SpeechVoice } from '@/domain/speech';
import { errorText } from '@/lib/errors';

export function useSpeech(api: SpeechGateway, desktop: boolean) {
  const [status, setStatus] = useState<SpeechStatus | null>(null);
  const [pending, setPending] = useState<'save' | 'preview' | 'download' | 'stop' | null>(null);
  const [error, setError] = useState('');
  const [saved, setSaved] = useState(false);
  const alive = useRef(true);
  const operation = useRef(false);
  const ownsPreview = useRef(false);
  const refresh = useCallback(async () => {
    const value = await api.status();
    if (alive.current) setStatus(value);
  }, [api]);
  useEffect(() => {
    alive.current = true;
    if (desktop)
      void refresh().catch((error) => {
        if (alive.current) setError(errorText(error));
      });
    return () => {
      alive.current = false;
      if (ownsPreview.current) void api.stop().catch(() => {});
    };
  }, [desktop, refresh]);
  useEffect(() => {
    if (!desktop || (!pending && (!status || status.phase === 'idle'))) return;
    let fetching = false;
    const timer = window.setInterval(() => {
      if (fetching) return;
      fetching = true;
      void refresh()
        .catch((error) => {
          if (alive.current) setError(errorText(error));
        })
        .finally(() => {
          fetching = false;
        });
    }, 500);
    return () => window.clearInterval(timer);
  }, [desktop, pending, status?.phase, refresh]);
  async function run(kind: NonNullable<typeof pending>, action: () => Promise<void>) {
    if (!desktop || operation.current) return;
    operation.current = true;
    setPending(kind);
    setError('');
    setSaved(false);
    try {
      if (kind === 'preview') ownsPreview.current = true;
      await action();
      await refresh();
      if (alive.current && kind === 'save') setSaved(true);
    } catch (error) {
      if (alive.current && !/cancelled/i.test(errorText(error))) setError(errorText(error));
    } finally {
      operation.current = false;
      if (alive.current) {
        setPending(null);
        await refresh().catch(() => {});
      }
    }
  }
  // Stopping must remain available while a long download request is pending.
  async function stop() {
    try {
      await api.stop();
      await refresh();
    } catch (error) {
      if (alive.current) setError(errorText(error));
    }
  }
  return {
    status,
    pending,
    error: error || status?.error || '',
    saved,
    save: (preferences: SpeechPreferences) => run('save', () => api.save(preferences)),
    preview: (preferences: SpeechPreferences) => run('preview', () => api.preview(preferences)),
    download: (voice: SpeechVoice) => run('download', () => api.download(voice)),
    stop,
  };
}
