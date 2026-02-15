import React from 'react';

interface Step2ApiKeyProps {
  apiKey: string;
  showApiKey: boolean;
  validating: boolean;
  validationResult: 'idle' | 'valid' | 'invalid';
  onApiKeyChange: (value: string) => void;
  onToggleVisibility: () => void;
  onValidate: () => void;
  onOpenGroq: () => void;
}

const Step2ApiKey: React.FC<Step2ApiKeyProps> = ({
  apiKey,
  showApiKey,
  validating,
  validationResult,
  onApiKeyChange,
  onToggleVisibility,
  onValidate,
  onOpenGroq,
}) => {
  return (
    <div className="setup-step-body">
      <h2 className="setup-step-title">Configure your Groq API key</h2>
      <p className="setup-step-subtitle">
        Groq offers a free account and excellent transcription speed.
      </p>

      <div className="setup-steps-list">
        <div className="setup-step-row">
          <span className="setup-step-number">1</span>
          <span>Open console.groq.com</span>
        </div>
        <div className="setup-step-row">
          <span className="setup-step-number">2</span>
          <span>Create a free account</span>
        </div>
        <div className="setup-step-row">
          <span className="setup-step-number">3</span>
          <span>Generate an API key and paste it below</span>
        </div>
      </div>

      <button type="button" className="setup-secondary-btn" onClick={onOpenGroq}>
        Open console.groq.com
      </button>

      <div className="setup-field">
        <label className="setup-label" htmlFor="setup-api-key">
          API key
        </label>
        <div className="setup-input-row">
          <input
            id="setup-api-key"
            className="setup-input setup-input-mono"
            value={apiKey}
            onChange={(event) => onApiKeyChange(event.target.value)}
            type={showApiKey ? 'text' : 'password'}
            placeholder="gsk_..."
            autoComplete="off"
          />
          <button type="button" className="setup-icon-btn" onClick={onToggleVisibility}>
            {showApiKey ? 'Hide' : 'Show'}
          </button>
        </div>
      </div>

      <div className="setup-validation">
        {validationResult === 'valid' && <span className="setup-valid">✓ Valid key</span>}
        {validationResult === 'invalid' && <span className="setup-invalid">✕ Invalid key</span>}
        {validationResult === 'idle' && (
          <span className="setup-muted">Validate once to continue with confidence.</span>
        )}
      </div>

      <button
        type="button"
        className="setup-primary-outline-btn"
        disabled={validating || !apiKey.trim()}
        onClick={onValidate}
      >
        {validating ? 'Validating...' : 'Validate key'}
      </button>
    </div>
  );
};

export default Step2ApiKey;
