import React, { useState, useRef } from 'react';
import { useRecording } from '../../hooks/useRecording';
import { useAudioLevel } from '../../hooks/useAudioLevel';
import Waveform from './Waveform';
import ModeToggle from './ModeToggle';
import RecordButton from './RecordButton';
import type { ToastPayload } from '../../types/toast';



const ZentraLogo: React.FC = () => (
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
    <circle cx="12" cy="12" r="10.5" stroke="rgba(255,255,255,0.85)" strokeWidth="1.5" />
    <path d="M8 12C8 9.79 9.79 8 12 8" stroke="rgba(255,255,255,0.55)" strokeWidth="1.5" strokeLinecap="round" />
    <path d="M12 8C14.21 8 16 9.79 16 12" stroke="rgba(255,255,255,0.85)" strokeWidth="1.5" strokeLinecap="round" />
    <path d="M16 12H12V12" stroke="rgba(255,255,255,0.85)" strokeWidth="1.5" strokeLinecap="round" />
    <circle cx="12" cy="12" r="1.5" fill="rgba(255,255,255,0.7)" />
  </svg>
);

const CancelButton: React.FC<{ onClick: () => void }> = React.memo(({ onClick }) => (
  <button
    className="cancel-btn"
    onClick={(e) => {
      e.stopPropagation();
      onClick();
    }}
    aria-label="Cancel"
  >
    <svg viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
      <line x1="1.5" y1="1.5" x2="8.5" y2="8.5" />
      <line x1="8.5" y1="1.5" x2="1.5" y2="8.5" />
    </svg>
  </button>
));

const ProcessingContent: React.FC = React.memo(() => (
  <div className="processing-content">
    <div className="spinner" />
    <span className="processing-text">Transcrevendo...</span>
  </div>
));

const FloatingBar: React.FC<{ onToast?: (toast: ToastPayload) => void }> = ({ onToast }) => {
  const [hovered, setHovered] = useState(false);
  const { state, mode, setMode, startRecording, stopRecording, cancel, closeApp } = useRecording({ onToast });
  const audioLevel = useAudioLevel(state === 'recording');
  const barRef = useRef<HTMLDivElement>(null);
  const isInteractiveState = state === 'idle' || state === 'recording';
  const showHoverControls = hovered && isInteractiveState;
  const barClass = `floating-bar ${state}${showHoverControls ? ' hovered' : ''}`;

  return (
    <div className="floating-bar-wrapper">
      <div
        ref={barRef}
        className={barClass}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
        onClick={() => {
          if (state === 'idle') startRecording();
        }}
        style={{ cursor: state === 'idle' ? 'pointer' : 'default' }}
      >
        <div className="bar-logo">
          <ZentraLogo />
          <span className="bar-brand">zentra</span>
        </div>

        <div className="bar-center">
          {isInteractiveState && (
            <Waveform audioLevel={audioLevel} isRecording={state === 'recording'} compact />
          )}
          {state === 'processing' && <ProcessingContent />}
        </div>

        {isInteractiveState && (
          <div className="bar-controls">
            {showHoverControls && <ModeToggle mode={mode} onChange={setMode} />}
            {state === 'recording' ? (
              <RecordButton variant="stop" onClick={stopRecording} />
            ) : (
              <RecordButton variant="start" onClick={startRecording} />
            )}
            {showHoverControls && (
              <CancelButton onClick={closeApp} />
            )}
          </div>
        )}

        {state === 'processing' && (
          <div className="bar-controls">
            <CancelButton onClick={cancel} />
          </div>
        )}
      </div>
    </div>
  );
};

export default FloatingBar;
