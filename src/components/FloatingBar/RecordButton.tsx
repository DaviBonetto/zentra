import React from 'react';

interface RecordButtonProps {
  variant: 'start' | 'stop';
  onClick: () => void;
}

const RecordButton: React.FC<RecordButtonProps> = React.memo(({ variant, onClick }) => (
  <button
    className={`record-btn ${variant}`}
    onClick={(e) => { e.stopPropagation(); onClick(); }}
    aria-label={variant === 'start' ? 'Start recording' : 'Stop recording'}
  >
    {variant === 'start' ? <div className="start-icon" /> : <div className="stop-icon" />}
  </button>
));

export default RecordButton;
