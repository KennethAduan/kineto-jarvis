import type { AssistantGateway } from '../../domain/ports';
import { Channel, invoke, isTauri } from '@tauri-apps/api/core';
import type {
  Activity,
  Bootstrap,
  Conversation,
  Message,
  Model,
  SendResult,
  StreamEvent,
} from '../../domain/models';

export const desktop = isTauri();
export const api: AssistantGateway = {
  bootstrap: () => invoke<Bootstrap>('bootstrap'),
  models: () => invoke<Model[]>('discover_models'),
  create: () => invoke<Conversation>('create_conversation'),
  messages: (id: string) => invoke<Message[]>('get_messages', { conversationId: id }),
  delete: (id: string) => invoke<void>('delete_conversation', { conversationId: id }),
  activity: () => invoke<Activity[]>('get_activity'),
  cancel: () => invoke<void>('cancel_generation'),
  send: (
    conversationId: string,
    content: string,
    model: string,
    onEvent: (event: StreamEvent) => void,
  ) => {
    const channel = new Channel<StreamEvent>();
    channel.onmessage = onEvent;
    return invoke<SendResult>('send_message', { conversationId, content, model, onEvent: channel });
  },
};
