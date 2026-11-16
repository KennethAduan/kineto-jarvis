import { act, renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { useVoice } from './useVoice';
import type { VoiceGateway, VoiceStatus } from '@/domain/voice';

function fixture(ready = true) {
  const status: VoiceStatus = {
    stage: 'idle',
    model: { ready, bytes: 147951465 },
    downloaded_bytes: 0,
    recording: { active: false, seconds: 0 },
    speaking: false,
  };
  const api: VoiceGateway = {
    status: vi.fn(async () => structuredClone(status)),
    start: vi.fn(async () => {
      status.stage = 'recording';
      status.recording.active = true;
    }),
    finish: vi.fn(async () => {
      status.stage = 'idle';
      return 'Recognized words';
    }),
    cancel: vi.fn(async () => {
      status.stage = 'idle';
      status.recording.active = false;
    }),
    download: vi.fn(async () => {
      status.model.ready = true;
    }),
    speak: vi.fn(async () => {
      status.speaking = true;
    }),
    stop: vi.fn(async () => {
      status.speaking = false;
    }),
  };
  return { api, status };
}

describe('voice controller', () => {
  it('requires explicit setup and never opens the microphone on mount', async () => {
    const { api } = fixture(false);
    const receive = vi.fn();
    const { result } = renderHook(() => useVoice(api, true, 'chat', receive));
    await waitFor(() => expect(result.current.status).not.toBeNull());
    expect(api.start).not.toHaveBeenCalled();
    await act(() => result.current.start());
    expect(result.current.setupOpen).toBe(true);
    expect(api.download).not.toHaveBeenCalled();
    expect(api.start).not.toHaveBeenCalled();
    expect(receive).not.toHaveBeenCalled();
  });
  it('returns dictation for review only after finishing', async () => {
    const { api } = fixture();
    const receive = vi.fn();
    const { result } = renderHook(() => useVoice(api, true, 'chat', receive));
    await waitFor(() => expect(result.current.status?.model.ready).toBe(true));
    await act(() => result.current.start());
    expect(result.current.dictating).toBe(true);
    expect(receive).not.toHaveBeenCalled();
    await act(() => result.current.finish());
    expect(receive).toHaveBeenCalledExactlyOnceWith('Recognized words');
    expect(result.current.dictating).toBe(false);
  });
  it('discards a late transcript when the selected conversation changes', async () => {
    const { api } = fixture();
    let resolve!: (text: string) => void;
    api.finish = vi.fn(
      () =>
        new Promise<string>((done) => {
          resolve = done;
        }),
    );
    const receive = vi.fn();
    const { result, rerender } = renderHook(({ id }) => useVoice(api, true, id, receive), {
      initialProps: { id: 'first' },
    });
    await waitFor(() => expect(result.current.status?.model.ready).toBe(true));
    await act(() => result.current.start());
    let finishing!: Promise<void>;
    act(() => {
      finishing = result.current.finish();
    });
    rerender({ id: 'second' });
    await act(async () => {
      resolve('Old conversation');
      await finishing;
    });
    expect(api.cancel).toHaveBeenCalled();
    expect(receive).not.toHaveBeenCalled();
  });
  it('reports microphone denial and remains usable', async () => {
    const { api } = fixture();
    api.start = vi.fn().mockRejectedValue(new Error('Microphone permission denied'));
    const { result } = renderHook(() => useVoice(api, true, 'chat', vi.fn()));
    await waitFor(() => expect(result.current.status?.model.ready).toBe(true));
    await act(() => result.current.start());
    expect(result.current.error).toContain('permission denied');
    expect(result.current.pending).toBeNull();
    expect(result.current.dictating).toBe(false);
  });
});
