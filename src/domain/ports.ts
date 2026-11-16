import type {
  Activity,
  Bootstrap,
  Conversation,
  Message,
  Model,
  SendResult,
  StreamEvent,
} from './models';

/** Application contract; the hook does not import Tauri or browser storage. */
export interface AssistantGateway {
  bootstrap(): Promise<Bootstrap>;
  models(): Promise<Model[]>;
  create(): Promise<Conversation>;
  messages(id: string): Promise<Message[]>;
  delete(id: string): Promise<void>;
  activity(): Promise<Activity[]>;
  cancel(): Promise<void>;
  send(
    conversationId: string,
    content: string,
    model: string,
    onEvent: (event: StreamEvent) => void,
  ): Promise<SendResult>;
}
