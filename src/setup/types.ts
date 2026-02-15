export type UseCase = 'work' | 'study' | 'creation' | 'general';

export interface SetupState {
  setupCompleted: boolean;
  userName: string;
  useCase: string;
  hasApiKey: boolean;
  inputDeviceName?: string | null;
  hotkey: string;
  language: 'pt' | 'en' | 'auto';
  githubUrl: string;
}

export interface CompleteSetupPayload {
  userName: string;
  useCase: string;
  apiKey: string;
  inputDeviceName?: string;
  hotkey: string;
  language: 'pt' | 'en' | 'auto';
}

export interface SaveSetupPartialPayload {
  userName?: string;
  useCase?: string;
  apiKey?: string;
  inputDeviceName?: string;
  hotkey?: string;
  language?: 'pt' | 'en' | 'auto';
}
