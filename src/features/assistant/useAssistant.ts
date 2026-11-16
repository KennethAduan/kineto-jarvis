import { useEffect, useRef, useState } from 'react';
import type { AssistantGateway } from '../../domain/ports';
import type { Activity, Bootstrap, Conversation, Message, Model } from '../../domain/models';
import { preferredModel } from '../../domain/preferred-model';
import { errorText } from '../../lib/errors';
import { useConversationSearch } from './useConversationSearch';
type Page = 'chat' | 'activity' | 'permissions';
export function useAssistant(api: AssistantGateway, desktop: boolean) {
  const [boot, setBoot] = useState<Bootstrap | null>(null);
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [messages, setMessages] = useState<Message[]>([]);
  const [activity, setActivity] = useState<Activity[]>([]);
  const [models, setModels] = useState<Model[]>([]);
  const [model, setModel] = useState('');
  const [connection, setConnection] = useState<'checking' | 'online' | 'offline'>('checking');
  const [connectionError, setConnectionError] = useState('');
  const [checkedAt, setCheckedAt] = useState<number | null>(null);
  const [error, setError] = useState('');
  const [selected, setSelected] = useState<string | null>(null);
  const [page, setPage] = useState<Page>('chat');
  const [draft, setDraft] = useState('');
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(false);
  const [thinking, setThinking] = useState(false);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const operation = useRef(false);
  const modelCheck = useRef(false);
  const selectionVersion = useRef(0);
  const textarea = useRef<HTMLTextAreaElement>(null);
  const title = conversations.find((c) => c.id === selected)?.title ?? 'Your personal intelligence';
  const history = useConversationSearch(api, conversations, desktop && boot !== null);

  async function checkConnection() {
    if (modelCheck.current) return;
    modelCheck.current = true;
    setConnection('checking');
    setConnectionError('');
    try {
      const found = await api.models();
      setModels(found);
      setModel((current) =>
        found.some((m) => m.id === current) ? current : preferredModel(found),
      );
      setConnection('online');
      if (!found.length)
        setConnectionError('Server connected. Load Qwen3.5-9B in LM Studio, then refresh.');
    } catch (e) {
      setConnection('offline');
      setModels([]);
      setModel('');
      setConnectionError(errorText(e));
    } finally {
      setCheckedAt(Math.floor(Date.now() / 1000));
      modelCheck.current = false;
    }
  }

  useEffect(() => {
    if (!desktop) {
      setConnection('offline');
      return;
    }
    let alive = true;
    api
      .bootstrap()
      .then((data) => {
        if (!alive) return;
        setBoot(data);
        setConversations(data.conversations);
        setActivity(data.activity);
      })
      .catch((e) => {
        if (alive) setError(errorText(e));
      });
    void checkConnection();
    const interval = window.setInterval(() => {
      if (!document.hidden && !operation.current) void checkConnection();
    }, 30000);
    return () => {
      alive = false;
      window.clearInterval(interval);
    };
  }, []);

  async function openConversation(id: string | null) {
    if (operation.current) return;
    const version = ++selectionVersion.current;
    setSelected(id);
    setPage('chat');
    setError('');
    setDeleteId(null);
    setMessages([]);
    if (!id) {
      setLoading(false);
      textarea.current?.focus();
      return;
    }
    setLoading(true);
    try {
      const history = await api.messages(id);
      if (selectionVersion.current === version) setMessages(history);
    } catch (e) {
      if (selectionVersion.current === version) setError(errorText(e));
    } finally {
      if (selectionVersion.current === version) setLoading(false);
    }
  }

  async function refreshHistory() {
    const data = await api.bootstrap();
    setConversations(data.conversations);
    setActivity(data.activity);
  }

  async function send() {
    const content = draft.trim();
    if (!content || operation.current || loading || !model || !boot || connection !== 'online')
      return;
    operation.current = true;
    setBusy(true);
    setError('');
    setThinking(false);
    let id = selected;
    let submitted = false;
    try {
      if (!id) {
        const conversation = await api.create();
        id = conversation.id;
        setSelected(id);
        setConversations((current) => [conversation, ...current]);
      }
      const conversationId = id;
      let assistantId: string | null = null;
      const result = await api.send(conversationId, content, model, (event) => {
        if (event.kind === 'started') {
          submitted = true;
          assistantId = event.message.id;
          setDraft('');
          const user: Message = {
            id: `pending-${event.message.id}`,
            conversation_id: conversationId,
            role: 'user',
            content,
            status: 'complete',
            created_at: event.message.created_at,
          };
          setMessages((current) => [...current, user, event.message]);
          void refreshHistory().catch(() => {});
        } else if (event.kind === 'delta') {
          setThinking(false);
          setMessages((current) =>
            current.map((m) =>
              m.id === assistantId ? { ...m, content: m.content + event.text } : m,
            ),
          );
        } else {
          setThinking(true);
        }
      });
      setMessages((current) =>
        current.map((m) => (m.id === result.message.id ? result.message : m)),
      );
      if (result.error) setError(result.error);
    } catch (e) {
      setError(errorText(e));
    } finally {
      // Read the source of truth even after an IPC/persistence error.
      try {
        if (id) setMessages(await api.messages(id));
        await refreshHistory();
      } catch (e) {
        setError(`History refresh failed: ${errorText(e)}`);
      }
      if (!submitted) setDraft(content);
      operation.current = false;
      setBusy(false);
      setThinking(false);
      textarea.current?.focus();
    }
  }

  async function removeConversation(id: string) {
    if (operation.current) return;
    try {
      await api.delete(id);
      if (selected === id) await openConversation(null);
      setDeleteId(null);
      await refreshHistory();
    } catch (e) {
      setError(errorText(e));
    }
  }

  async function cancel() {
    try {
      await api.cancel();
    } catch (e) {
      setError(errorText(e));
    }
  }

  return {
    history,
    boot,
    conversations,
    messages,
    activity,
    models,
    model,
    setModel,
    connection,
    connectionError,
    checkedAt,
    error,
    setError,
    selected,
    page,
    setPage,
    draft,
    setDraft,
    busy,
    loading,
    thinking,
    deleteId,
    setDeleteId,
    textarea,
    title,
    checkConnection,
    openConversation,
    send,
    removeConversation,
    cancel,
  };
}
export type AssistantController = ReturnType<typeof useAssistant>;
