import { useState, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';

/**
 * Listens for 'audio-level' events from Tauri backend.
 * Returns a smoothed audio level (0.0 - 1.0).
 * When not active, returns 0 and uses a simulated idle animation.
 */
export function useAudioLevel(active: boolean): number {
  const [level, setLevel] = useState(0);
  const smoothedRef = useRef(0);
  const rafRef = useRef<number>(0);

  useEffect(() => {
    if (!active) {
      setLevel(0);
      smoothedRef.current = 0;
      return;
    }

    let unlisten: (() => void) | null = null;
    let hasRealData = false;

    // Listen for real audio level events from backend
    const setupListener = async () => {
      try {
        const unlistenFn = await listen<number>('audio-level', (event) => {
          hasRealData = true;
          const raw = Math.max(0, Math.min(1, event.payload));
          // Exponential smoothing
          smoothedRef.current += (raw - smoothedRef.current) * 0.35;
          setLevel(smoothedRef.current);
        });
        unlisten = unlistenFn;
      } catch (err) {
        console.warn('audio-level listener failed:', err);
      }
    };

    setupListener();

    // Simulated fallback if no real data comes within 500ms
    const fallbackTimer = setTimeout(() => {
      if (!hasRealData) {
        const simulate = () => {
          if (!active) return;
          const t = Date.now() * 0.003;
          const sim = 0.15
            + Math.sin(t) * 0.15
            + Math.sin(t * 2.7) * 0.1
            + Math.random() * 0.08;
          setLevel(Math.max(0, Math.min(1, sim)));
          rafRef.current = requestAnimationFrame(simulate);
        };
        simulate();
      }
    }, 500);

    return () => {
      clearTimeout(fallbackTimer);
      cancelAnimationFrame(rafRef.current);
      unlisten?.();
    };
  }, [active]);

  return level;
}
