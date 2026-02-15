import React from 'react';

interface SetupCompleteProps {
  name: string;
  hotkey: string;
}

const SetupComplete: React.FC<SetupCompleteProps> = ({ name, hotkey }) => {
  const displayHotkey = hotkey
    .split('+')
    .map((segment) => (segment === 'CommandOrControl' ? 'Ctrl' : segment))
    .join(' + ');

  return (
    <div className="setup-complete">
      <div className="setup-complete-icon">Z</div>
      <h2>All set, {name || 'there'}.</h2>
      <p>Use {displayHotkey} to start dictating with Zentra.</p>
      <p className="setup-muted">Zentra is now running in your tray.</p>
    </div>
  );
};

export default SetupComplete;
