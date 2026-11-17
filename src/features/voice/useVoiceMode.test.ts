import { act, renderHook, waitFor } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { useVoiceMode } from './useVoiceMode';
import type { VoiceModeGateway, VoiceModeEvent } from '@/domain/voice-mode';
import type { VoiceGateway } from '@/domain/voice';
import type { AssistantController } from '@/features/assistant/useAssistant';
import type { Message } from '@/domain/models';
function fixture() {
  let receive!: (event: VoiceModeEvent) => void;
  const api: VoiceModeGateway = {
    status: vi.fn(async () => ({
      enabled: false,
      phase: 'off' as const,
      model: { ready: true, bytes: 6682588 },
      downloaded_bytes: 0,
      error: null,
    })),
    download: vi.fn(async () => {}),
    start: vi.fn(async (callback) => {
      receive = callback;
      callback({ kind: 'phase', phase: 'waiting' });
      return 'session';
    }),
    heartbeat: vi.fn(async () => {}),
    resume: vi.fn(async () => {}),
    stop: vi.fn(async () => {}),
  };
  const message: Message = {
    id: 'reply',
    conversation_id: 'chat',
    role: 'assistant',
    content: 'Four.',
    status: 'complete',
    created_at: 1,
  };
  const assistant = {
    selected: 'chat',
    page: 'chat',
    ensureConversation: vi.fn(async () => 'chat'),
    send: vi.fn(async () => message),
  } as unknown as AssistantController;
  const voice = {
    speak: vi.fn(async () => {}),
    status: vi.fn(async () => ({
      stage: 'idle',
      model: { ready: true, bytes: 1 },
      downloaded_bytes: 0,
      recording: { active: false, seconds: 0 },
      speaking: false,
    })),
  } as unknown as VoiceGateway;
  return { api, assistant, voice, message, emit: (event: VoiceModeEvent) => receive(event) };
}
describe('hands-free voice conversation', () => {
  it('starts only on request, sends the transcript, speaks the saved reply, then resumes listening', async () => {
    const { api, assistant, voice, emit } = fixture();
    const { result } = renderHook(() => useVoiceMode(api, voice, assistant, vi.fn()));
    expect(api.start).not.toHaveBeenCalled();
    await act(() => result.current.start());
    expect(result.current.enabled).toBe(true);
    expect(assistant.send).not.toHaveBeenCalled();
    act(() => emit({ kind: 'transcript', text: 'What is two plus two?' }));
    await waitFor(() => expect(voice.speak).toHaveBeenCalledWith('chat', 'reply'));
    expect(assistant.send).toHaveBeenCalledWith('What is two plus two?');
    await waitFor(() => expect(api.resume).toHaveBeenCalledWith('session'));
    await act(() => result.current.stop());
    expect(result.current.enabled).toBe(false);
  });
  it('never plays a late reply after the user stops voice mode', async () => {
    const { api, assistant, voice, message, emit } = fixture();
    let finish!: (message: Message) => void;
    assistant.send = vi.fn(
      () =>
        new Promise<Message>((resolve) => {
          finish = resolve;
        }),
    );
    const { result } = renderHook(() => useVoiceMode(api, voice, assistant, vi.fn()));
    await act(() => result.current.start());
    act(() => emit({ kind: 'transcript', text: 'A slow request' }));
    await act(() => result.current.stop());
    await act(async () => {
      finish(message);
    });
    expect(voice.speak).not.toHaveBeenCalled();
    expect(api.resume).not.toHaveBeenCalled();
  });
});
