import { invoke } from '@tauri-apps/api/core';
import type { SpeechGateway, SpeechStatus } from '@/domain/speech';
export const speechApi: SpeechGateway = {
  status: () => invoke<SpeechStatus>('speech_status'),
  save: (preferences) => invoke<void>('speech_save_preferences', { preferences }),
  preview: (preferences) => invoke<void>('speech_preview', { preferences }),
  download: (voice) => invoke<void>('speech_download_model', { voice }),
  stop: () => invoke<void>('voice_stop_speaking'),
};
