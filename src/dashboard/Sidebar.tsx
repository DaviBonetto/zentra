import React from 'react';

type Section = 'dashboard' | 'history' | 'settings' | 'community';

interface SidebarProps {
  activeSection: Section;
  appVersion: string;
  onChangeSection: (section: Section) => void;
}

const Sidebar: React.FC<SidebarProps> = ({ activeSection, appVersion, onChangeSection }) => {
  return (
    <aside className="dashboard-sidebar">
      <div className="dashboard-brand">
        <div className="dashboard-brand-icon">Z</div>
        <div>
          <div className="dashboard-brand-name">zentra</div>
          <div className="dashboard-brand-caption">voice desktop</div>
        </div>
      </div>

      <nav className="dashboard-nav">
        <button
          type="button"
          className={`dashboard-nav-btn ${activeSection === 'dashboard' ? 'active' : ''}`}
          onClick={() => onChangeSection('dashboard')}
        >
          Dashboard
        </button>
        <button
          type="button"
          className={`dashboard-nav-btn ${activeSection === 'history' ? 'active' : ''}`}
          onClick={() => onChangeSection('history')}
        >
          History
        </button>
        <button
          type="button"
          className={`dashboard-nav-btn ${activeSection === 'settings' ? 'active' : ''}`}
          onClick={() => onChangeSection('settings')}
        >
          Settings
        </button>
        <button
          type="button"
          className={`dashboard-nav-btn ${activeSection === 'community' ? 'active' : ''}`}
          onClick={() => onChangeSection('community')}
        >
          Contribute
        </button>
      </nav>

      <div className="dashboard-version">v{appVersion || '1.0.0'}</div>
    </aside>
  );
};

export default Sidebar;
