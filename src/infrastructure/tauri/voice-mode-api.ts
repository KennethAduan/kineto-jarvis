import { Channel, invoke } from '@tauri-apps/api/core';
import type { VoiceModeGateway, VoiceModeEvent, VoiceModeStatus } from '@/domain/voice-mode';
export const voiceModeApi: VoiceModeGateway = {
  status: () => invoke<VoiceModeStatus>('voice_mode_status'),
  download: () => invoke<void>('voice_mode_download'),
  start: (onEvent) => {
    const channel = new Channel<VoiceModeEvent>();
    channel.onmessage = onEvent;
    return invoke<string>('voice_mode_start', { onEvent: channel });
  },
  heartbeat: (sessionId) => invoke<void>('voice_mode_heartbeat', { sessionId }),
  resume: (sessionId) => invoke<void>('voice_mode_resume', { sessionId }),
  stop: () => invoke<void>('voice_mode_stop'),
};
