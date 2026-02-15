import React from 'react';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import App from '../App';
import SetupWizard from '../setup/SetupWizard';
import Dashboard from '../dashboard/Dashboard';

function detectWindowLabel(): string {
  try {
    const win = getCurrentWebviewWindow();
    return win.label;
  } catch {
    return 'main';
  }
}

const WindowRoot: React.FC = () => {
  const label = detectWindowLabel();

  if (label === 'setup') {
    return <SetupWizard />;
  }

  if (label === 'dashboard') {
    return <Dashboard />;
  }

  return <App />;
};

export default WindowRoot;
