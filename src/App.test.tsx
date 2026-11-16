import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App';
import { api } from './infrastructure/tauri/assistant-api';
import { preferredModel } from './domain/preferred-model';
import type { Message } from './domain/models';

vi.mock('./infrastructure/tauri/assistant-api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./infrastructure/tauri/assistant-api')>()),
  desktop: true,
  api: {
    bootstrap: vi.fn(),
    models: vi.fn(),
    messages: vi.fn(),
    create: vi.fn(),
    send: vi.fn(),
    activity: vi.fn(),
    delete: vi.fn(),
    cancel: vi.fn(),
  },
}));
const message: Message = {
  id: 'a',
  conversation_id: 'c',
  role: 'assistant',
  content: 'Hello locally.',
  status: 'complete',
  created_at: 1,
};
const boot = {
  conversations: [],
  activity: [],
  platform: { name: 'Windows', automation_available: false },
  permissions: [
    { capability: 'Files & shell', state: 'disabled', description: 'No commands registered.' },
  ],
  endpoint: 'http://127.0.0.1:1234/v1',
};
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.bootstrap).mockResolvedValue(boot);
  vi.mocked(api.models).mockResolvedValue([{ id: 'qwen/qwen3.5-9b' }]);
  vi.mocked(api.messages).mockResolvedValue([message]);
  vi.mocked(api.activity).mockResolvedValue([]);
  vi.mocked(api.cancel).mockResolvedValue(undefined);
  vi.mocked(api.create).mockResolvedValue({ id: 'c', title: 'New conversation', updated_at: 1 });
});
afterEach(cleanup);
describe('assistant workflow', () => {
  it('selects Qwen from discovery rather than hard-coding the request model', () => {
    expect(
      preferredModel([{ id: 'other' }, { id: 'lmstudio-community/Qwen3.5-9B-GGUF' }]),
    ).toContain('Qwen');
    expect(preferredModel([])).toBe('');
  });
  it('streams a response, stops generation, and reloads durable history', async () => {
    let finish: (() => void) | undefined;
    vi.mocked(api.send).mockImplementation(async (_id, _text, _model, onEvent) => {
      onEvent({ kind: 'started', message: { ...message, content: '', status: 'streaming' } });
      onEvent({ kind: 'delta', text: 'Hello locally.' });
      await new Promise<void>((resolve) => {
        finish = resolve;
      });
      return { message, error: null };
    });
    render(<App />);
    await waitFor(() =>
      expect(screen.getByLabelText('ACTIVE MODEL').getAttribute('disabled')).toBeNull(),
    );
    fireEvent.change(screen.getByLabelText('Message JARVIS'), { target: { value: 'Hello' } });
    await waitFor(() =>
      expect((screen.getByLabelText('Send message') as HTMLButtonElement).disabled).toBe(false),
    );
    fireEvent.click(screen.getByLabelText('Send message'));
    await screen.findByText('Hello locally.');
    expect(api.send).toHaveBeenCalledWith('c', 'Hello', 'qwen/qwen3.5-9b', expect.any(Function));
    fireEvent.click(screen.getByLabelText('Stop generation'));
    expect(api.cancel).toHaveBeenCalledOnce();
    await act(async () => {
      finish?.();
    });
    await waitFor(() => expect(api.messages).toHaveBeenCalledWith('c'));
  });
  it('keeps a draft and prevents sending while LM Studio is offline', async () => {
    vi.mocked(api.models).mockRejectedValue(new Error('Server offline'));
    render(<App />);
    await screen.findByText('Server offline');
    fireEvent.change(screen.getByLabelText('Message JARVIS'), { target: { value: 'keep this' } });
    expect((screen.getByLabelText('Send message') as HTMLButtonElement).disabled).toBe(true);
    fireEvent.keyDown(screen.getByLabelText('Message JARVIS'), { key: 'Enter' });
    expect(api.send).not.toHaveBeenCalled();
    expect((screen.getByLabelText('Message JARVIS') as HTMLTextAreaElement).value).toBe(
      'keep this',
    );
    vi.mocked(api.models).mockResolvedValue([{ id: 'qwen/qwen3.5-9b' }]);
    fireEvent.click(screen.getByRole('button', { name: 'Check again' }));
    await waitFor(() =>
      expect((screen.getByLabelText('Send message') as HTMLButtonElement).disabled).toBe(false),
    );
    expect((screen.getByLabelText('Message JARVIS') as HTMLTextAreaElement).value).toBe(
      'keep this',
    );
  });
  it('opens a focused confirmation dialog before deleting a conversation', async () => {
    vi.mocked(api.bootstrap).mockResolvedValue({
      ...boot,
      conversations: [{ id: 'c', title: 'Saved', updated_at: 1 }],
    });
    render(<App />);
    fireEvent.click(await screen.findByLabelText('Delete Saved'));
    expect(screen.getByRole('alertdialog')).toBeDefined();
    expect(api.delete).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Keep conversation' }));
    await waitFor(() => expect(screen.queryByRole('alertdialog')).toBeNull());
    expect(api.delete).not.toHaveBeenCalled();
  });
  it('renders model text as plain text, never executable HTML', async () => {
    vi.mocked(api.bootstrap).mockResolvedValue({
      ...boot,
      conversations: [{ id: 'c', title: 'Saved', updated_at: 1 }],
    });
    vi.mocked(api.messages).mockResolvedValue([
      { ...message, content: '<img src=x onerror="alert(1)">' },
    ]);
    render(<App />);
    fireEvent.click(await screen.findByTitle('Saved'));
    await screen.findByText('<img src=x onerror="alert(1)">');
    expect(document.querySelector('.message-content img')).toBeNull();
  });
});
