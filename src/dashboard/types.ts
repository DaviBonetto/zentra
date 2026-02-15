export interface HistoryItem {
  id: string;
  text: string;
  timestamp: string;
  durationSeconds: number;
  wordCount: number;
}

export interface DashboardStats {
  totalTranscriptions: number;
  totalWords: number;
  minutesSaved: number;
  wpm: number;
}

export interface DashboardData {
  userName: string;
  hasApiKey: boolean;
  apiKeyMasked?: string | null;
  hotkey: string;
  language: 'pt' | 'en' | 'auto';
  stats: DashboardStats;
  history: HistoryItem[];
  githubUrl: string;
  appVersion: string;
}
