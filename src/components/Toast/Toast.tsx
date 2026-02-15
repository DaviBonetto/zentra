import React, { useEffect, useState } from 'react';
import type { ToastPayload, ToastType } from '../../types/toast';

interface ToastState extends ToastPayload {
  id: number;
}

interface ToastProps {
  toast: ToastState | null;
}

const Toast: React.FC<ToastProps> = ({ toast }) => {
  const [visible, setVisible] = useState(false);
  const [currentType, setCurrentType] = useState<ToastType>('copied');
  const [currentTitle, setCurrentTitle] = useState('');
  const [currentSubtitle, setCurrentSubtitle] = useState<string | undefined>();
  const [hideDelayMs, setHideDelayMs] = useState(1700);

  useEffect(() => {
    if (toast) {
      const duration = toast.durationMs ?? 2000;
      const hideDelay = Math.max(0, duration - 300);

      setCurrentType(toast.type);
      setCurrentTitle(toast.title);
      setCurrentSubtitle(toast.subtitle);
      setHideDelayMs(hideDelay);
      setVisible(true);
      const timer = setTimeout(() => setVisible(false), duration);
      return () => clearTimeout(timer);
    }

    setVisible(false);
    return undefined;
  }, [toast]);

  if (!visible) return null;

  return (
    <div className="toast-container">
      <div
        className={`toast ${currentType === 'pasted' ? 'toast-pasted' : ''}`}
        style={{ '--toast-hide-delay': `${hideDelayMs}ms` } as React.CSSProperties}
      >
        <div className="toast-title">{currentTitle}</div>
        {currentSubtitle && <div className="toast-subtitle">{currentSubtitle}</div>}
      </div>
    </div>
  );
};

export default Toast;
