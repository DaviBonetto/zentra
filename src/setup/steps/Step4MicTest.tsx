import React, { useMemo } from 'react';
import { useAudioLevel } from '../../hooks/useAudioLevel';

interface Step4MicTestProps {
  micAvailable: boolean;
  micName: string;
  monitoring: boolean;
  onRetryDetect: () => void;
}

const BAR_COUNT = 24;

const Step4MicTest: React.FC<Step4MicTestProps> = ({
  micAvailable,
  micName,
  monitoring,
  onRetryDetect,
}) => {
  const level = useAudioLevel(monitoring);

  const bars = useMemo(() => {
    const now = Date.now() * 0.01;
    return Array.from({ length: BAR_COUNT }).map((_, index) => {
      const centerDistance = Math.abs(index - (BAR_COUNT - 1) / 2) / ((BAR_COUNT - 1) / 2);
      const centerFactor = 1 - centerDistance * 0.7;
      const wave = Math.sin(now + index * 0.7) * 1.5;
      const amplitude = micAvailable ? level * 26 : 0;
      const height = Math.max(4, Math.min(34, 4 + amplitude * centerFactor + wave));
      return height;
    });
  }, [level, micAvailable]);

  const volumePct = Math.round(level * 100);
  const meterClass = volumePct < 45 ? 'low' : volumePct < 75 ? 'mid' : 'high';

  return (
    <div className="setup-step-body">
      <h2 className="setup-step-title">Test your microphone</h2>
      <p className="setup-step-subtitle">Speak naturally and confirm Zentra receives your voice.</p>

      <div className="setup-mic-wave">
        {bars.map((height, index) => (
          <div key={index} className="setup-mic-bar" style={{ height }} />
        ))}
      </div>

      {micAvailable ? (
        <div className="setup-mic-status success">Microphone detected · {micName || 'Default input'}</div>
      ) : (
        <div className="setup-mic-status error">
          No microphone found. Check system permissions and try again.
        </div>
      )}

      <div className="setup-volume-row">
        <span>Volume</span>
        <div className={`setup-volume-meter ${meterClass}`}>
          <div className="setup-volume-fill" style={{ width: `${volumePct}%` }} />
        </div>
        <span>{volumePct}%</span>
      </div>

      {!micAvailable && (
        <button type="button" className="setup-primary-outline-btn" onClick={onRetryDetect}>
          Retry detection
        </button>
      )}
    </div>
  );
};

export default Step4MicTest;
