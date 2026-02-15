import React, { useMemo } from 'react';

interface Step3HotkeyProps {
  hotkey: string;
  capturing: boolean;
  warning?: string | null;
  onStartCapture: () => void;
  onStopCapture: () => void;
  onResetDefault: () => void;
  onCaptureKeyDown: (event: React.KeyboardEvent<HTMLDivElement>) => void;
}

function formatHotkeyDisplay(hotkey: string): string[] {
  if (!hotkey) return [];
  return hotkey.split('+').map((segment) => {
    if (segment === 'CommandOrControl') return 'Ctrl';
    return segment;
  });
}

const Step3Hotkey: React.FC<Step3HotkeyProps> = ({
  hotkey,
  capturing,
  warning,
  onStartCapture,
  onStopCapture,
  onResetDefault,
  onCaptureKeyDown,
}) => {
  const keys = useMemo(() => formatHotkeyDisplay(hotkey), [hotkey]);

  return (
    <div className="setup-step-body">
      <h2 className="setup-step-title">Choose your hotkey</h2>
      <p className="setup-step-subtitle">
        Press your preferred shortcut to start and stop dictation.
      </p>

      <div
        className={`setup-hotkey-capture ${capturing ? 'active' : ''}`}
        onClick={capturing ? onStopCapture : onStartCapture}
        onKeyDown={onCaptureKeyDown}
        role="button"
        tabIndex={0}
      >
        <p className="setup-hotkey-hint">
          {capturing ? 'Press keys now...' : 'Click here to capture shortcut'}
        </p>
        <div className="setup-hotkey-preview">
          {keys.length === 0 && <span className="setup-muted">Ctrl + Shift + Space</span>}
          {keys.map((key) => (
            <span key={key} className="setup-kbd-pill">
              {key}
            </span>
          ))}
        </div>
      </div>

      {warning ? <div className="setup-warning">{warning}</div> : null}

      <button type="button" className="setup-link" onClick={onResetDefault}>
        Use default: Ctrl+Shift+Space
      </button>
    </div>
  );
};

export default Step3Hotkey;
