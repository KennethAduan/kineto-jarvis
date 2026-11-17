export type SpeechVoice = 'bm_george' | 'bm_fable' | 'bm_daniel' | 'bm_lewis' | 'system';
export interface SpeechPreferences {
  voice: SpeechVoice;
  speed: number;
}
export interface SpeechStatus {
  preferences: SpeechPreferences;
  model: { ready: boolean; bytes: number };
  phase: 'idle' | 'downloading' | 'preparing' | 'playing';
  downloaded_bytes: number;
  error: string | null;
}
export interface SpeechGateway {
  status(): Promise<SpeechStatus>;
  save(preferences: SpeechPreferences): Promise<void>;
  preview(preferences: SpeechPreferences): Promise<void>;
  download(): Promise<void>;
  stop(): Promise<void>;
}
export const speechVoices: { id: SpeechVoice; label: string }[] = [
  { id: 'bm_george', label: 'George · British male' },
  { id: 'bm_fable', label: 'Fable · British male' },
  { id: 'bm_daniel', label: 'Daniel · British male' },
  { id: 'bm_lewis', label: 'Lewis · British male' },
  { id: 'system', label: 'System default voice' },
];
