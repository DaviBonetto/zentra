import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { openUrl } from '@tauri-apps/plugin-opener';
import SetupComplete from './SetupComplete';
import Step1Welcome from './steps/Step1Welcome';
import Step2ApiKey from './steps/Step2ApiKey';
import Step3Hotkey from './steps/Step3Hotkey';
import Step4MicTest from './steps/Step4MicTest';
import type {
  CompleteSetupPayload,
  SaveSetupPartialPayload,
  SetupState,
  UseCase,
} from './types';

type ValidationResult = 'idle' | 'valid' | 'invalid';

const DEFAULT_HOTKEY = 'CommandOrControl+Shift+Space';

function normalizeHotkeyPart(key: string): string | null {
  if (!key || ['Control', 'Shift', 'Alt', 'Meta'].includes(key)) {
    return null;
  }
  if (key === ' ') return 'Space';
  if (key.length === 1) return key.toUpperCase();
  if (key === 'Escape') return 'Esc';
  if (key.startsWith('Arrow')) return key;
  return key;
}

const SetupWizard: React.FC = () => {
  const [loading, setLoading] = useState(true);
  const [step, setStep] = useState(1);
  const [showComplete, setShowComplete] = useState(false);

  const [githubUrl, setGithubUrl] = useState('https://github.com/DaviBonetto/zentra');
  const [userName, setUserName] = useState('');
  const [useCase, setUseCase] = useState<UseCase>('general');
  const [apiKey, setApiKey] = useState('');
  const [hasSavedApiKey, setHasSavedApiKey] = useState(false);
  const [showApiKey, setShowApiKey] = useState(false);
  const [validatingKey, setValidatingKey] = useState(false);
  const [validationResult, setValidationResult] = useState<ValidationResult>('idle');
  const [hotkey, setHotkey] = useState(DEFAULT_HOTKEY);
  const [language, setLanguage] = useState<'pt' | 'en' | 'auto'>('pt');
  const [capturing, setCapturing] = useState(false);
  const [hotkeyWarning, setHotkeyWarning] = useState<string | null>(null);
  const [micAvailable, setMicAvailable] = useState(false);
  const [micName, setMicName] = useState('');
  const [micMonitoring, setMicMonitoring] = useState(false);
  const [inputDevices, setInputDevices] = useState<string[]>([]);
  const [selectedInputDevice, setSelectedInputDevice] = useState<string>('');
  const [refreshingDevices, setRefreshingDevices] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const progressPct = useMemo(() => (step / 4) * 100, [step]);

  useEffect(() => {
    let mounted = true;
    const load = async () => {
      try {
        const setupState = await invoke<SetupState>('get_setup_state');
        if (!mounted) return;
        setUserName(setupState.userName || '');
        setUseCase((setupState.useCase as UseCase) || 'general');
        setHotkey(setupState.hotkey || DEFAULT_HOTKEY);
        setLanguage(setupState.language || 'pt');
        setSelectedInputDevice(setupState.inputDeviceName || '');
        setHasSavedApiKey(setupState.hasApiKey);
        setGithubUrl(setupState.githubUrl || githubUrl);
      } catch (error) {
        if (!mounted) return;
        setErrorMessage(String(error));
      } finally {
        if (mounted) setLoading(false);
      }
    };
    void load();
    return () => {
      mounted = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const persistPartial = useCallback(async (payload: SaveSetupPartialPayload) => {
    await invoke('save_setup_partial', { payload });
  }, []);

  const detectMic = useCallback(async () => {
    try {
      const info = await invoke<{ available: boolean; name?: string }>('get_microphone_info');
      const available = Boolean(info.available);
      const name = info.name || '';
      setMicAvailable(available);
      setMicName(name);
      return { available, name };
    } catch {
      setMicAvailable(false);
      setMicName('');
      return { available: false, name: '' };
    }
  }, []);

  const loadInputDevices = useCallback(async () => {
    setRefreshingDevices(true);
    try {
      const response = await invoke<{ devices: string[]; selected?: string | null }>(
        'list_input_devices',
      );
      setInputDevices(response.devices || []);
      if (response.selected) {
        setSelectedInputDevice(response.selected);
      }
      return response;
    } catch (error) {
      setErrorMessage(`Unable to list microphones: ${String(error)}`);
      return { devices: [] as string[], selected: null as string | null };
    } finally {
      setRefreshingDevices(false);
    }
  }, []);

  useEffect(() => {
    if (step !== 4) {
      void invoke('stop_mic_monitor');
      setMicMonitoring(false);
      return;
    }

    let cancelled = false;
    const bootMicMonitor = async () => {
      const devicesResponse = await loadInputDevices();
      const micInfo = await detectMic();
      if (cancelled) return;

      if (!selectedInputDevice && micInfo.name && devicesResponse.devices.includes(micInfo.name)) {
        try {
          await invoke('select_input_device', { name: micInfo.name });
          setSelectedInputDevice(micInfo.name);
        } catch (error) {
          setErrorMessage(`Failed to auto-select microphone: ${String(error)}`);
        }
      }

      if (!micInfo.available || cancelled) return;

      try {
        await invoke('start_mic_monitor');
        if (!cancelled) {
          setMicMonitoring(true);
        }
      } catch (error) {
        if (!cancelled) {
          setMicMonitoring(false);
          setErrorMessage(`Microphone test failed: ${String(error)}`);
        }
      }
    };

    void bootMicMonitor();
    return () => {
      cancelled = true;
      void invoke('stop_mic_monitor');
      setMicMonitoring(false);
    };
  }, [step, detectMic, loadInputDevices, selectedInputDevice]);

  const handleOpenGithub = useCallback(async () => {
    await openUrl(githubUrl);
  }, [githubUrl]);

  const handleOpenGroq = useCallback(async () => {
    await openUrl('https://console.groq.com');
  }, []);

  const validateKey = useCallback(async () => {
    setValidatingKey(true);
    setValidationResult('idle');
    setErrorMessage(null);
    try {
      const valid = await invoke<boolean>('validate_groq_key', { apiKey });
      setValidationResult(valid ? 'valid' : 'invalid');
      if (valid) {
        setHasSavedApiKey(true);
      }
    } catch (error) {
      setValidationResult('invalid');
      setErrorMessage(String(error));
    } finally {
      setValidatingKey(false);
    }
  }, [apiKey]);

  const handleCaptureKeyDown = useCallback((event: React.KeyboardEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.stopPropagation();

    const parts: string[] = [];
    if (event.ctrlKey || event.metaKey) parts.push('CommandOrControl');
    if (event.shiftKey) parts.push('Shift');
    if (event.altKey) parts.push('Alt');

    const normalized = normalizeHotkeyPart(event.key);
    if (normalized) {
      parts.push(normalized);
    }

    if (parts.length < 2) {
      return;
    }

    const nextHotkey = parts.join('+');
    setHotkey(nextHotkey);
    setCapturing(false);
    if (nextHotkey === 'CommandOrControl+Space') {
      setHotkeyWarning('Ctrl+Space can conflict with IME on some systems.');
    } else {
      setHotkeyWarning(null);
    }
  }, []);

  const goNext = useCallback(async () => {
    setErrorMessage(null);
    if (submitting) return;

    if (step === 1) {
      if (!userName.trim()) {
        setErrorMessage('Please enter your name.');
        return;
      }
      await persistPartial({ userName, useCase });
      setStep(2);
      return;
    }

    if (step === 2) {
      if (!apiKey.trim() && !hasSavedApiKey) {
        setErrorMessage('Please provide your Groq API key.');
        return;
      }

      if (apiKey.trim() && validationResult !== 'valid') {
        setErrorMessage('Validate your API key before continuing.');
        return;
      }

      await persistPartial({ apiKey: apiKey.trim() ? apiKey : undefined });
      setStep(3);
      return;
    }

    if (step === 3) {
      await persistPartial({ hotkey, language });
      setStep(4);
      return;
    }

    if (step === 4) {
      if (!micAvailable) {
        setErrorMessage('A microphone is required to finish setup.');
        return;
      }
      setSubmitting(true);
      try {
        const payload: CompleteSetupPayload = {
          userName: userName.trim(),
          useCase,
          apiKey: apiKey.trim(),
          inputDeviceName: selectedInputDevice || micName || undefined,
          hotkey,
          language,
        };
        await invoke('complete_setup', { payload });
        setShowComplete(true);

        setTimeout(() => {
          const current = getCurrentWebviewWindow();
          void current.hide();
        }, 2500);
      } catch (error) {
        setErrorMessage(String(error));
      } finally {
        setSubmitting(false);
      }
    }
  }, [
    apiKey,
    hasSavedApiKey,
    hotkey,
    language,
    micAvailable,
    persistPartial,
    step,
    submitting,
    selectedInputDevice,
    useCase,
    userName,
    validationResult,
  ]);

  const goBack = useCallback(() => {
    if (submitting) return;
    if (step > 1) {
      setStep((prev) => prev - 1);
    }
  }, [step, submitting]);

  if (loading) {
    return (
      <div className="setup-window">
        <div className="setup-card">
          <p className="setup-muted">Loading setup...</p>
        </div>
      </div>
    );
  }

  if (showComplete) {
    return (
      <div className="setup-window">
        <div className="setup-card">
          <SetupComplete name={userName} hotkey={hotkey} />
        </div>
      </div>
    );
  }

  return (
    <div className="setup-window">
      <div className="setup-card">
        <div className="setup-progress-track">
          <div className="setup-progress-fill" style={{ width: `${progressPct}%` }} />
        </div>

        <div className="setup-step-meta">
          <span>Step {step} of 4</span>
        </div>

        {step === 1 && (
          <Step1Welcome
            userName={userName}
            useCase={useCase}
            githubUrl={githubUrl}
            onUserNameChange={setUserName}
            onUseCaseChange={setUseCase}
            onOpenGithub={handleOpenGithub}
          />
        )}

        {step === 2 && (
          <Step2ApiKey
            apiKey={apiKey}
            showApiKey={showApiKey}
            validating={validatingKey}
            validationResult={validationResult}
            onApiKeyChange={(value) => {
              setApiKey(value);
              setValidationResult('idle');
            }}
            onToggleVisibility={() => setShowApiKey((prev) => !prev)}
            onValidate={validateKey}
            onOpenGroq={handleOpenGroq}
          />
        )}

        {step === 3 && (
          <Step3Hotkey
            hotkey={hotkey}
            capturing={capturing}
            warning={hotkeyWarning}
            onStartCapture={() => setCapturing(true)}
            onStopCapture={() => setCapturing(false)}
            onResetDefault={() => {
              setHotkey(DEFAULT_HOTKEY);
              setHotkeyWarning(null);
            }}
            onCaptureKeyDown={handleCaptureKeyDown}
          />
        )}

        {step === 4 && (
          <Step4MicTest
            micAvailable={micAvailable}
            micName={micName}
            monitoring={micMonitoring}
            inputDevices={inputDevices}
            selectedInputDevice={selectedInputDevice}
            refreshingDevices={refreshingDevices}
            onRetryDetect={() => {
              void loadInputDevices();
              void detectMic();
            }}
            onSelectInputDevice={(name) => {
              void (async () => {
                try {
                  await invoke('select_input_device', { name: name || null });
                  setSelectedInputDevice(name);
                  if (step === 4) {
                    await invoke('stop_mic_monitor');
                    setMicMonitoring(false);
                    const micInfo = await detectMic();
                    if (micInfo.available) {
                      await invoke('start_mic_monitor');
                      setMicMonitoring(true);
                    }
                  }
                } catch (error) {
                  setErrorMessage(`Failed to select microphone: ${String(error)}`);
                }
              })();
            }}
          />
        )}

        {errorMessage ? <div className="setup-error">{errorMessage}</div> : null}

        <div className="setup-actions">
          <button type="button" className="setup-back-btn" onClick={goBack} disabled={step === 1}>
            Back
          </button>
          <button type="button" className="setup-next-btn" onClick={goNext} disabled={submitting}>
            {step === 4 ? (submitting ? 'Finishing...' : 'Finish setup') : 'Continue'}
          </button>
        </div>
      </div>
    </div>
  );
};

export default SetupWizard;
