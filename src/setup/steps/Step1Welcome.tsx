import React from 'react';
import type { UseCase } from '../types';

interface Step1WelcomeProps {
  userName: string;
  useCase: UseCase;
  githubUrl: string;
  onUserNameChange: (value: string) => void;
  onUseCaseChange: (value: UseCase) => void;
  onOpenGithub: () => void;
}

const useCaseOptions: Array<{ id: UseCase; label: string }> = [
  { id: 'work', label: 'Work and productivity' },
  { id: 'study', label: 'Study and research' },
  { id: 'creation', label: 'Content and creation' },
  { id: 'general', label: 'General use' },
];

const Step1Welcome: React.FC<Step1WelcomeProps> = ({
  userName,
  useCase,
  githubUrl,
  onUserNameChange,
  onUseCaseChange,
  onOpenGithub,
}) => {
  return (
    <div className="setup-step-body">
      <div className="setup-brand-block">
        <div className="setup-brand-icon">Z</div>
        <h1 className="setup-brand-title">zentra</h1>
        <p className="setup-brand-subtitle">Free, fast voice-to-text for your daily flow.</p>
      </div>

      <div className="setup-field">
        <label className="setup-label" htmlFor="setup-name">
          What is your name?
        </label>
        <input
          id="setup-name"
          className="setup-input"
          value={userName}
          onChange={(event) => onUserNameChange(event.target.value)}
          placeholder="Your name..."
          autoComplete="off"
        />
      </div>

      <div className="setup-field">
        <label className="setup-label">What will you use Zentra for?</label>
        <div className="setup-usecase-grid">
          {useCaseOptions.map((option) => (
            <button
              key={option.id}
              type="button"
              className={`setup-usecase-pill ${useCase === option.id ? 'active' : ''}`}
              onClick={() => onUseCaseChange(option.id)}
            >
              {option.label}
            </button>
          ))}
        </div>
      </div>

      <button type="button" className="setup-link" onClick={onOpenGithub}>
        Give us a star on GitHub ({githubUrl})
      </button>
    </div>
  );
};

export default Step1Welcome;
