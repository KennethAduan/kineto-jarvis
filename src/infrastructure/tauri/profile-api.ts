import { invoke } from '@tauri-apps/api/core';
import type { PersonalProfile, ProfileGateway } from '@/domain/profile';
export const profileApi: ProfileGateway = {
  get: () => invoke<PersonalProfile>('profile_get'),
  save: (profile) => invoke<void>('profile_save', { profile }),
};
