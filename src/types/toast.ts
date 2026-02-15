export type ToastType = 'pasted' | 'copied' | 'error';

export interface ToastPayload {
  type: ToastType;
  title: string;
  subtitle?: string;
  durationMs?: number;
}
