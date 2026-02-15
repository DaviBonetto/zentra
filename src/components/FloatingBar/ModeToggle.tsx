import React from 'react';

interface ModeToggleProps {
  mode: 'ai' | 'clarity';
  onChange: (mode: 'ai' | 'clarity') => void;
}

const ModeToggle: React.FC<ModeToggleProps> = React.memo(({ mode, onChange }) => (
  <div className="mode-toggle">
    <button
      className={`mode-btn${mode === 'ai' ? ' active' : ''}`}
      onClick={(e) => { e.stopPropagation(); onChange('ai'); }}
    >
      IA
    </button>
    <button
      className={`mode-btn${mode === 'clarity' ? ' active' : ''}`}
      onClick={(e) => { e.stopPropagation(); onChange('clarity'); }}
    >
      CL
    </button>
  </div>
));

export default ModeToggle;
