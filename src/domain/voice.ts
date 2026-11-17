export type VoiceStage = 'idle' | 'recording' | 'transcribing' | 'downloading';
export interface VoiceStatus {
  stage: VoiceStage;
  model: { ready: boolean; bytes: number };
  downloaded_bytes: number;
  recording: { active: boolean; seconds: number };
  speaking: boolean;
  playback_error?: string | null;
}
/** Voice is independently replaceable; no microphone or speech API lives in presentation. */
export interface VoiceGateway {
  status(): Promise<VoiceStatus>;
  download(): Promise<void>;
  start(): Promise<void>;
  finish(): Promise<string>;
  cancel(): Promise<void>;
  speak(conversationId: string, messageId: string): Promise<void>;
  stop(): Promise<void>;
}
