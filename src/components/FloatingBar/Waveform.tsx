import React, { useEffect, useRef } from 'react';

interface WaveformProps {
  audioLevel: number;
  isRecording: boolean;
  compact?: boolean;
}

const Waveform: React.FC<WaveformProps> = React.memo(
  ({ audioLevel, isRecording, compact = true }) => {
    const barCount = compact ? 8 : 14;
    const barsRef = useRef<(HTMLDivElement | null)[]>([]);
    const rafRef = useRef<number>(0);
    const prevLevel = useRef(0);

    useEffect(() => {
      const animate = () => {
        const target = Math.max(0, Math.min(1, audioLevel));
        const smoothing = isRecording ? 0.36 : 0.12;
        prevLevel.current += (target - prevLevel.current) * smoothing;
        const smoothed = prevLevel.current;
        const now = performance.now();
        const minHeight = 4;
        const maxHeight = compact ? 24 : 28;

        barsRef.current.forEach((bar, i) => {
          if (!bar) return;
          const centerDist = Math.abs(i - (barCount - 1) / 2) / ((barCount - 1) / 2);
          const centerFactor = 1 - centerDist * 0.65;

          let height = minHeight;
          let opacity = 0.58;

          if (isRecording) {
            const wave =
              Math.sin(now * 0.018 + i * 0.7) * 0.45 +
              Math.sin(now * 0.009 + i * 1.3) * 0.35;
            const dynamic = minHeight + (smoothed * 20 + wave * 3) * centerFactor;
            height = Math.min(maxHeight, Math.max(minHeight, dynamic));
            opacity = Math.min(0.95, Math.max(0.58, 0.6 + smoothed * 0.34 + wave * 0.04));
          } else {
            const idle =
              0.5 + Math.sin(now * 0.006 + i * 0.52) * 0.26 + Math.sin(now * 0.0035 + i * 0.19) * 0.16;
            height = Math.min(12, Math.max(minHeight, minHeight + idle * 5 * centerFactor));
            opacity = Math.min(0.82, Math.max(0.55, 0.56 + idle * 0.12));
          }

          bar.style.height = `${height}px`;
          bar.style.opacity = `${opacity}`;
        });

        rafRef.current = requestAnimationFrame(animate);
      };

      rafRef.current = requestAnimationFrame(animate);
      return () => cancelAnimationFrame(rafRef.current);
    }, [audioLevel, barCount, compact, isRecording]);

    return (
      <div className={`waveform ${compact ? 'waveform-compact' : ''}`}>
        {Array.from({ length: barCount }).map((_, i) => (
          <div
            key={i}
            ref={(el) => {
              barsRef.current[i] = el;
            }}
            className="waveform-bar"
          />
        ))}
      </div>
    );
  },
);

export default Waveform;
