import { useEffect, useRef, useState } from 'react';
import { Download } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { VoiceModeGateway, VoiceModeStatus } from '@/domain/voice-mode';
import { errorText } from '@/lib/errors';
export function WakeSettings({ api, desktop }: { api: VoiceModeGateway; desktop: boolean }) {
  const [status, setStatus] = useState<VoiceModeStatus | null>(null),
    [pending, setPending] = useState(false),
    [error, setError] = useState('');
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    if (desktop)
      void api
        .status()
        .then((value) => {
          if (alive.current) setStatus(value);
        })
        .catch((error) => {
          if (alive.current) setError(errorText(error));
        });
    return () => {
      alive.current = false;
    };
  }, [api, desktop]);
  const downloading = pending || status?.phase === 'downloading';
  useEffect(() => {
    if (!downloading) return;
    let busy = false;
    const timer = window.setInterval(() => {
      if (busy) return;
      busy = true;
      void api
        .status()
        .then((value) => {
          if (alive.current) setStatus(value);
        })
        .catch(() => {})
        .finally(() => {
          busy = false;
        });
    }, 500);
    return () => window.clearInterval(timer);
  }, [api, downloading]);
  async function download() {
    setPending(true);
    setError('');
    try {
      await api.download();
    } catch (error) {
      if (alive.current && !/cancelled/i.test(errorText(error))) setError(errorText(error));
    } finally {
      if (alive.current) {
        setPending(false);
        await api
          .status()
          .then(setStatus)
          .catch(() => {});
      }
    }
  }
  return (
    <section className="speech-settings" aria-labelledby="wake-heading">
      <h3 id="wake-heading">“Hey Jarvis” voice mode</h3>
      <p>
        Wake-word and speech detection run locally. Enable Voice mode in the workspace, say “Hey
        Jarvis,” then ask your question. Replies play aloud and the microphone reopens for your next
        turn.
      </p>
      <p>
        Voice mode sends spoken requests automatically to your local model and saves them in chat
        history. It stays off until you enable it, and stops when minimized or closed.
      </p>
      <Button
        variant="outline"
        disabled={!desktop || !status || downloading || status.enabled}
        onClick={() => void download()}
      >
        <Download className="size-4" aria-hidden="true" />
        {status?.model.ready ? 'Reinstall listening pack' : 'Download listening pack · 7 MB'}
      </Button>
      {status?.model.ready && (
        <small className="voice-ready">Hey Jarvis listening pack ready.</small>
      )}
      {downloading && (
        <>
          <progress
            aria-label="Listening pack download progress"
            value={status?.downloaded_bytes ?? 0}
            max={status?.model.bytes ?? 6682588}
          />
          <Button
            variant="ghost"
            onClick={() => void api.stop().catch((error) => setError(errorText(error)))}
          >
            Cancel listening download
          </Button>
        </>
      )}
      {error && (
        <p role="alert" className="voice-error">
          {error}
        </p>
      )}
      <small>
        Dictation and a spoken-reply voice must also be ready. Speak after the listening cue; the
        microphone pauses during replies.
      </small>
    </section>
  );
}
