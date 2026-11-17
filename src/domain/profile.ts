export interface PersonalProfile {
  display_name: string;
  content: string;
  enabled: boolean;
}
export interface ProfileGateway {
  get(): Promise<PersonalProfile>;
  save(profile: PersonalProfile): Promise<void>;
}
