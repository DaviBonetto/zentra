import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { openUrl } from '@tauri-apps/plugin-opener';
import Sidebar from './Sidebar';
import StatsBar from './sections/StatsBar';
import History from './sections/History';
import type { DashboardData } from './types';

type Section = 'dashboard' | 'history' | 'settings' | 'community';

interface SettingsDraft {
  userName: string;
  apiKey: string;
  hotkey: string;
  language: 'pt' | 'en' | 'auto';
}

const INSPIRATION_MESSAGES = [
  'Ready to shape the future',
  'Your voice is faster than typing',
  'Capture ideas while they are fresh',
  'Dictate, build, and ship',
];

const Dashboard: React.FC = () => {
  const [loading, setLoading] = useState(true);
  const [activeSection, setActiveSection] = useState<Section>('dashboard');
  const [data, setData] = useState<DashboardData | null>(null);
  const [settingsDraft, setSettingsDraft] = useState<SettingsDraft>({
    userName: '',
    apiKey: '',
    hotkey: 'CommandOrControl+Shift+Space',
    language: 'pt',
  });
  const [notice, setNotice] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [messageIndex, setMessageIndex] = useState(0);
  const [isDashboardMaximized, setIsDashboardMaximized] = useState(false);

  const loadDashboard = useCallback(async () => {
    setLoading(true);
    try {
      const result = await invoke<DashboardData>('get_dashboard_data');
      setData(result);
      setSettingsDraft({
        userName: result.userName || '',
        apiKey: '',
        hotkey: result.hotkey || 'CommandOrControl+Shift+Space',
        language: result.language || 'pt',
      });
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadDashboard();
  }, [loadDashboard]);

  useEffect(() => {
    let unlistenNavigate: (() => void) | null = null;
    let unlistenRefresh: (() => void) | null = null;
    let unlistenHistory: (() => void) | null = null;

    void listen<string>('dashboard:navigate', (event) => {
      if (event.payload === 'settings') {
        setActiveSection('settings');
      } else {
        setActiveSection('dashboard');
      }
      void loadDashboard();
    }).then((fn) => {
      unlistenNavigate = fn;
    });

    void listen('dashboard:refresh', () => {
      void loadDashboard();
    }).then((fn) => {
      unlistenRefresh = fn;
    });

    void listen('dashboard:history-updated', () => {
      void loadDashboard();
    }).then((fn) => {
      unlistenHistory = fn;
    });

    return () => {
      unlistenNavigate?.();
      unlistenRefresh?.();
      unlistenHistory?.();
    };
  }, [loadDashboard]);

  useEffect(() => {
    const timer = window.setInterval(() => {
      setMessageIndex((index) => (index + 1) % INSPIRATION_MESSAGES.length);
    }, 3_600_000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    const interval = window.setInterval(() => {
      void loadDashboard();
    }, 15000);

    const handleFocus = () => {
      void loadDashboard();
    };

    window.addEventListener('focus', handleFocus);
    return () => {
      window.clearInterval(interval);
      window.removeEventListener('focus', handleFocus);
    };
  }, [loadDashboard]);

  const showNotice = useCallback((message: string) => {
    setNotice(message);
    setTimeout(() => {
      setNotice((current) => (current === message ? null : current));
    }, 1800);
  }, []);

  const totalItems = data?.history.length ?? 0;
  const displayName = data?.userName?.trim() || 'Creator';

  const pageTitle = useMemo(() => {
    if (activeSection === 'history') return 'History';
    if (activeSection === 'settings') return 'Settings';
    if (activeSection === 'community') return 'Contribute to Zentra';
    return `Press ${data?.hotkey?.replace('CommandOrControl', 'Ctrl') ?? 'Ctrl+Shift+Space'} to dictate`;
  }, [activeSection, data?.hotkey]);

  const inspirationalLine = useMemo(() => {
    const base = INSPIRATION_MESSAGES[messageIndex];
    const name = data?.userName?.trim();
    return name ? `${base}, ${name}.` : `${base}.`;
  }, [data?.userName, messageIndex]);

  const handleDeleteHistory = useCallback(
    async (id: string) => {
      await invoke('delete_history_item', { id });
      await loadDashboard();
      showNotice('History item deleted');
    },
    [loadDashboard, showNotice],
  );

  const handleClearHistory = useCallback(async () => {
    await invoke('clear_history');
    await loadDashboard();
    showNotice('History cleared');
  }, [loadDashboard, showNotice]);

  const handleSaveSettings = useCallback(async () => {
    if (!data) return;
    setSaving(true);
    try {
      await invoke('update_settings', {
        payload: {
          userName: settingsDraft.userName,
          hotkey: settingsDraft.hotkey,
          language: settingsDraft.language,
          apiKey: settingsDraft.apiKey.trim() ? settingsDraft.apiKey : undefined,
        },
      });
      await loadDashboard();
      setSettingsDraft((current) => ({ ...current, apiKey: '' }));
      showNotice('Settings updated');
    } finally {
      setSaving(false);
    }
  }, [data, loadDashboard, settingsDraft, showNotice]);

  if (loading) {
    return (
      <div className="dashboard-window">
        <div className="dashboard-shell">
          <p className="setup-muted">Loading dashboard...</p>
        </div>
      </div>
    );
  }

  if (!data) {
    return (
      <div className="dashboard-window">
        <div className="dashboard-shell">
          <p className="setup-muted">Unable to load dashboard data.</p>
        </div>
      </div>
    );
  }

  return (
    <div className="dashboard-window">
      <div className="dashboard-shell">
        <Sidebar
          activeSection={activeSection}
          appVersion={data.appVersion}
          onChangeSection={setActiveSection}
        />

        <main className="dashboard-main">
          <header className="dashboard-header">
            <div>
              <p className="dashboard-inspiration">{inspirationalLine}</p>
              <h1>{pageTitle}</h1>
              <p>{totalItems} transcriptions saved</p>
            </div>
            <div className="dashboard-window-controls">
              <button
                type="button"
                className="dashboard-window-btn"
                aria-label="Minimize dashboard"
                onClick={() => {
                  void invoke('dashboard_minimize');
                }}
              >
                −
              </button>
              <button
                type="button"
                className="dashboard-window-btn"
                aria-label="Toggle maximize dashboard"
                onClick={async () => {
                  const maximized = await invoke<boolean>('dashboard_toggle_maximize');
                  setIsDashboardMaximized(maximized);
                }}
              >
                {isDashboardMaximized ? '❐' : '□'}
              </button>
              <button
                type="button"
                className="dashboard-window-btn dashboard-window-btn-close"
                aria-label="Close dashboard"
                onClick={() => {
                  void invoke('dashboard_close');
                }}
              >
                ×
              </button>
            </div>
          </header>

          {activeSection === 'dashboard' && (
            <section className="dashboard-overview">
              <div className="dashboard-badges">
                <span className="dashboard-badge">{`User: ${displayName}`}</span>
                <span className="dashboard-badge">
                  {data.hasApiKey ? `API: ${data.apiKeyMasked ?? 'configured'}` : 'API key missing'}
                </span>
              </div>
              <StatsBar stats={data.stats} />
              <div className="dashboard-history-panel">
                <h2 className="dashboard-section-title">Recent history</h2>
                <History
                  items={data.history.slice(0, 6)}
                  onDelete={handleDeleteHistory}
                  onCopied={() => showNotice('Copied to clipboard')}
                />
              </div>
            </section>
          )}

          {activeSection === 'history' && (
            <>
              <h2 className="dashboard-section-title">All transcriptions</h2>
              <History
                items={data.history}
                onDelete={handleDeleteHistory}
                onCopied={() => showNotice('Copied to clipboard')}
              />
            </>
          )}

          {activeSection === 'settings' && (
            <div className="dashboard-settings">
              <div className="setup-field">
                <label className="setup-label">User name</label>
                <input
                  className="setup-input"
                  value={settingsDraft.userName}
                  onChange={(event) => setSettingsDraft((current) => ({ ...current, userName: event.target.value }))}
                />
              </div>

              <div className="setup-field">
                <label className="setup-label">Groq API key</label>
                <input
                  className="setup-input setup-input-mono"
                  value={settingsDraft.apiKey}
                  onChange={(event) => setSettingsDraft((current) => ({ ...current, apiKey: event.target.value }))}
                  placeholder={
                    data.hasApiKey
                      ? `Stored key: ${data.apiKeyMasked ?? 'configured'} (leave blank to keep)`
                      : 'gsk_...'
                  }
                  autoComplete="off"
                />
              </div>

              <div className="setup-field">
                <label className="setup-label">Hotkey</label>
                <input
                  className="setup-input setup-input-mono"
                  value={settingsDraft.hotkey}
                  onChange={(event) => setSettingsDraft((current) => ({ ...current, hotkey: event.target.value }))}
                />
              </div>

              <div className="setup-field">
                <label className="setup-label">Recognition language</label>
                <div className="setup-usecase-grid">
                  {[
                    { id: 'pt', label: 'Portuguese (pt)' },
                    { id: 'en', label: 'English (en)' },
                    { id: 'auto', label: 'Automatic' },
                  ].map((option) => (
                    <button
                      key={option.id}
                      type="button"
                      className={`setup-usecase-pill ${settingsDraft.language === option.id ? 'active' : ''}`}
                      onClick={() =>
                        setSettingsDraft((current) => ({
                          ...current,
                          language: option.id as SettingsDraft['language'],
                        }))
                      }
                    >
                      {option.label}
                    </button>
                  ))}
                </div>
              </div>

              <div className="dashboard-settings-actions">
                <button type="button" className="setup-primary-outline-btn" onClick={handleSaveSettings} disabled={saving}>
                  {saving ? 'Saving...' : 'Save settings'}
                </button>
                <button type="button" className="dashboard-danger-btn" onClick={handleClearHistory}>
                  Clear history
                </button>
              </div>

              <div className="dashboard-links">
                <button type="button" className="dashboard-link-btn" onClick={() => void openUrl(data.githubUrl)}>
                  GitHub repository
                </button>
                <button
                  type="button"
                  className="dashboard-link-btn"
                  onClick={() => void openUrl(`${data.githubUrl}/issues/new/choose`)}
                >
                  Report a bug
                </button>
              </div>
            </div>
          )}

          {activeSection === 'community' && (
            <div className="dashboard-community">
              <div className="dashboard-community-mission-card">
                <div className="dashboard-community-mission-icon">Z</div>
                <p className="dashboard-community-mission-text">
                  Build the biggest open-source voice dictation app in the world. That is Zentra&apos;s mission.
                </p>
              </div>

              <div className="dashboard-community-vertical">
                <div className="dashboard-community-step">
                  <span className="dashboard-community-step-index">01</span>
                  <p>Fork the repository.</p>
                </div>
                <div className="dashboard-community-step">
                  <span className="dashboard-community-step-index">02</span>
                  <p>Create your branch and implement high-impact improvements.</p>
                </div>
                <div className="dashboard-community-step">
                  <span className="dashboard-community-step-index">03</span>
                  <p>Open a pull request with context and tests on changed files.</p>
                </div>
                <div className="dashboard-community-step">
                  <span className="dashboard-community-step-index">04</span>
                  <p>Davi Bonetto reviews and merges high-impact contributions.</p>
                </div>
              </div>

              <p className="dashboard-community-quote">
                &quot;Your contributions power our mission. Let&apos;s shape the future of open source together.&quot;
              </p>

              <div className="dashboard-community-actions">
                <button type="button" className="dashboard-link-btn" onClick={() => void openUrl(data.githubUrl)}>
                  Fork on GitHub
                </button>
                <button
                  type="button"
                  className="dashboard-link-btn"
                  onClick={() => void openUrl(`${data.githubUrl}/pulls`)}
                >
                  Open Pull Requests
                </button>
                <button
                  type="button"
                  className="dashboard-link-btn dashboard-link-btn-wide"
                  onClick={() => void openUrl(`${data.githubUrl}/issues/new/choose`)}
                >
                  Suggest an improvement
                </button>
              </div>

              <div className="dashboard-community-footer-links">
                <button
                  type="button"
                  className="dashboard-community-text-link"
                  onClick={() => void openUrl(`${data.githubUrl}/blob/main/CONTRIBUTING.md`)}
                >
                  Contribution Guidelines
                </button>
                <span>•</span>
                <button
                  type="button"
                  className="dashboard-community-text-link"
                  onClick={() => void openUrl(`${data.githubUrl}/discussions`)}
                >
                  Open Discussions
                </button>
              </div>
            </div>
          )}
        </main>
      </div>

      {notice && <div className="dashboard-notice">{notice}</div>}
    </div>
  );
};

export default Dashboard;
