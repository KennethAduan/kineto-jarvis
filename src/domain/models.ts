export interface Conversation {
  id: string;
  title: string;
  updated_at: number;
}
export interface Message {
  id: string;
  conversation_id: string;
  role: 'user' | 'assistant';
  content: string;
  status: string;
  created_at: number;
}
export interface Activity {
  id: string;
  conversation_id: string | null;
  model: string;
  status: string;
  detail: string;
  created_at: number;
}
export interface Permission {
  capability: string;
  state: string;
  description: string;
}
export interface Bootstrap {
  conversations: Conversation[];
  activity: Activity[];
  platform: { name: string; automation_available: boolean };
  permissions: Permission[];
  endpoint: string;
}
export interface Model {
  id: string;
}
export type StreamEvent =
  { kind: 'started'; message: Message } | { kind: 'delta'; text: string } | { kind: 'thinking' };
export interface SendResult {
  message: Message;
  error: string | null;
}
