import { invoke } from '@tauri-apps/api/core';
import type { VoiceGateway, VoiceStatus } from '@/domain/voice';
export const voiceApi: VoiceGateway = {
  status: () => invoke<VoiceStatus>('voice_status'),
  download: () => invoke<void>('voice_download_model'),
  start: () => invoke<void>('voice_start_recording'),
  finish: () => invoke<string>('voice_finish_recording'),
  cancel: () => invoke<void>('voice_cancel'),
  speak: (conversationId, messageId) =>
    invoke<void>('voice_speak_message', { conversationId, messageId }),
  stop: () => invoke<void>('voice_stop_speaking'),
};
