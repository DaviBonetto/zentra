import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import type { ToastPayload } from '../types/toast';

type BarState = 'idle' | 'recording' | 'processing';
const MAX_SEGMENT_SECONDS = 59;

interface UseRecordingOptions {
  onToast?: (toast: ToastPayload) => void;
}

interface AudioPayload {
  samples: number[];
  sample_rate: number;
  channels: number;
}

interface StitchedResult {
  full_text: string;
  total_duration_secs?: number;
}

interface PasteAttempt {
  pasted: boolean;
  reason?: string | null;
}

function splitAudioIntoChunks(audio: AudioPayload): AudioPayload[] {
  const sampleRate = Math.max(1, audio.sample_rate || 16000);
  const channels = Math.max(1, audio.channels || 1);
  const samplesPerSecond = sampleRate * channels;
  const maxSamplesPerChunk = samplesPerSecond * MAX_SEGMENT_SECONDS;

  if (!Array.isArray(audio.samples) || audio.samples.length <= maxSamplesPerChunk) {
    return [audio];
  }

  const chunks: AudioPayload[] = [];
  for (let start = 0; start < audio.samples.length; start += maxSamplesPerChunk) {
    const end = Math.min(start + maxSamplesPerChunk, audio.samples.length);
    chunks.push({
      samples: audio.samples.slice(start, end),
      sample_rate: sampleRate,
      channels,
    });
  }

  return chunks;
}

function estimateDurationSeconds(audio: AudioPayload): number {
  if (!audio.samples?.length || !audio.sample_rate || !audio.channels) {
    return 0;
  }
  const samplesPerSecond = Math.max(1, audio.sample_rate * audio.channels);
  return audio.samples.length / samplesPerSecond;
}

function countWords(text: string): number {
  return text.trim().split(/\s+/).filter(Boolean).length;
}

export function useRecording({ onToast }: UseRecordingOptions = {}) {
  const [state, setState] = useState<BarState>('idle');
  const [mode, setMode] = useState<'ai' | 'clarity'>('ai');
  const stateRef = useRef<BarState>('idle');
  const transitionLockRef = useRef(false);
  const listenerBoundRef = useRef(false);

  useEffect(() => {
    // keep latest state in ref so global shortcut callbacks never use stale values
    stateRef.current = state;
  }, [state]);

  const startRecording = useCallback(async () => {
    if (stateRef.current !== 'idle' || transitionLockRef.current) return;
    transitionLockRef.current = true;
    try {
      await invoke<string>('start_recording_session');
      await invoke('start_recording');
      setState('recording');
    } catch (err) {
      console.error('Start recording failed:', err);
      setState('idle');
    } finally {
      transitionLockRef.current = false;
    }
  }, []);

  const stopRecording = useCallback(async () => {
    if (stateRef.current !== 'recording' || transitionLockRef.current) return;
    transitionLockRef.current = true;
    setState('processing');
    try {
      const audio = await invoke<AudioPayload>('stop_recording');
      if (!audio?.samples?.length) {
        onToast?.({
          type: 'error',
          title: 'No audio captured',
          durationMs: 2200,
        });
        return;
      }

      const chunks = splitAudioIntoChunks(audio);
      for (const chunk of chunks) {
        await invoke('add_audio_segment', { audio: chunk });
      }

      const result = await invoke<StitchedResult>('finalize_recording_session');
      const finalText = result.full_text?.trim() ?? '';
      console.log('TRANSCRIPT:', finalText);
      if (!finalText) {
        onToast?.({
          type: 'error',
          title: 'No speech detected',
          durationMs: 2200,
        });
        return;
      }

      const durationSeconds = result.total_duration_secs ?? estimateDurationSeconds(audio);
      const wordCount = countWords(finalText);
      try {
        await invoke('record_transcription_history', {
          payload: {
            text: finalText,
            durationSeconds,
            wordCount,
            timestamp: new Date().toISOString(),
          },
        });
      } catch (historyError) {
        console.warn('History record failed:', historyError);
      }

      await writeText(finalText);
      const pasteResult = await invoke<PasteAttempt>('paste_text');

      if (pasteResult.pasted) {
        onToast?.({
          type: 'pasted',
          title: 'Pasted',
          durationMs: 1800,
        });
      } else {
        if (pasteResult.reason) {
          console.debug('Auto-paste fallback:', pasteResult.reason);
        }
        onToast?.({
          type: 'copied',
          title: 'Copied • Press Ctrl+V',
          durationMs: 2500,
        });
      }
    } catch (err) {
      console.error('Stop/transcribe failed:', err);
      onToast?.({
        type: 'error',
        title: 'Failed to process audio',
        durationMs: 2200,
      });
    } finally {
      setState('idle');
      transitionLockRef.current = false;
    }
  }, [onToast]);

  const cancel = useCallback(async () => {
    try {
      if (stateRef.current === 'recording') {
        await invoke('stop_recording').catch(() => {});
      }
    } catch (_) {}
    transitionLockRef.current = false;
    setState('idle');
  }, []);

  const handleToggleFromHotkey = useCallback(() => {
    if (transitionLockRef.current) return;

    if (stateRef.current === 'idle') {
      void startRecording();
    } else if (stateRef.current === 'recording') {
      void stopRecording();
    }
  }, [startRecording, stopRecording]);

  const closeApp = useCallback(async () => {
    try {
      if (stateRef.current === 'recording') {
        await invoke('stop_recording').catch(() => {});
      }
    } catch (_) {}
    transitionLockRef.current = false;
    setState('idle');
    await invoke('hide_main_window').catch(() => {});
  }, []);

  useEffect(() => {
    if (listenerBoundRef.current) return;
    listenerBoundRef.current = true;

    let disposed = false;
    let unlistenFn: (() => void) | null = null;
    void listen('toggle-recording', () => {
      handleToggleFromHotkey();
    })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        unlistenFn = unlisten;
      })
      .catch((err) => {
        console.warn('toggle-recording listener failed:', err);
      });

    return () => {
      disposed = true;
      listenerBoundRef.current = false;
      if (unlistenFn) {
        unlistenFn();
      }
    };
  }, [handleToggleFromHotkey]);

  return { state, mode, setMode, startRecording, stopRecording, cancel, closeApp };
}



