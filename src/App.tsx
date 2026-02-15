import { useState, useCallback } from 'react';
import { FloatingBar } from './components/FloatingBar';
import Toast from './components/Toast/Toast';
import type { ToastPayload } from './types/toast';

interface ToastState extends ToastPayload {
  id: number;
}

function App() {
  const [toast, setToast] = useState<ToastState | null>(null);

  const handleToast = useCallback((payload: ToastPayload) => {
    setToast({
      id: Date.now() + Math.floor(Math.random() * 1000),
      ...payload,
    });
  }, []);

  return (
    <>
      <FloatingBar onToast={handleToast} />
      <Toast toast={toast} />
    </>
  );
}

export default App;
