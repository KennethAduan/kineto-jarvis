export type VoiceModePhase =
  | 'off'
  | 'downloading'
  | 'starting'
  | 'waiting'
  | 'listening'
  | 'transcribing'
  | 'paused'
  | 'stopping';
export interface VoiceModeStatus {
  enabled: boolean;
  phase: VoiceModePhase;
  model: { ready: boolean; bytes: number };
  downloaded_bytes: number;
  error: string | null;
}
export type VoiceModeEvent =
  | { kind: 'phase'; phase: VoiceModePhase }
  | { kind: 'transcript'; text: string }
  | { kind: 'error'; message: string }
  | { kind: 'stopped' };
export interface VoiceModeGateway {
  status(): Promise<VoiceModeStatus>;
  download(): Promise<void>;
  start(onEvent: (event: VoiceModeEvent) => void): Promise<string>;
  heartbeat(sessionId: string): Promise<void>;
  resume(sessionId: string): Promise<void>;
  stop(): Promise<void>;
}
