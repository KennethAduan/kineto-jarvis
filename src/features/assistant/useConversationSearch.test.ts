import { act, cleanup, renderHook, waitFor } from '@testing-library/react';
import { afterEach, expect, it, vi } from 'vitest';
import type { AssistantGateway } from '@/domain/ports';
import type { Conversation } from '@/domain/models';
import { useConversationSearch } from './useConversationSearch';
afterEach(cleanup);

it('ignores an old search response after the query changes and restores history when cleared', async () => {
  const saved: Conversation[] = [{ id: 'saved', title: 'Saved', updated_at: 1 }];
  let oldResolve: ((value: Conversation[]) => void) | undefined;
  const search = vi.fn().mockImplementation((query: string) =>
    query === 'old'
      ? new Promise((resolve) => {
          oldResolve = resolve;
        })
      : Promise.resolve([{ id: 'new', title: 'New match', updated_at: 2 }]),
  );
  const gateway = { search } as unknown as AssistantGateway;
  const { result } = renderHook(() => useConversationSearch(gateway, saved, true));
  act(() => result.current.setQuery('old'));
  await waitFor(() => expect(search).toHaveBeenCalledWith('old'));
  act(() => result.current.setQuery('new'));
  await waitFor(() => expect(result.current.conversations[0]?.id).toBe('new'));
  await act(async () => oldResolve?.([{ id: 'old', title: 'Old result', updated_at: 0 }]));
  expect(result.current.conversations[0]?.id).toBe('new');
  act(() => result.current.setQuery(''));
  expect(result.current.conversations).toBe(saved);
});
