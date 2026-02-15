import React from 'react';
import type { DashboardStats } from '../types';

interface StatsBarProps {
  stats: DashboardStats;
}

function formatNumber(value: number): string {
  return new Intl.NumberFormat().format(Math.round(value));
}

function formatDecimal(value: number): string {
  return new Intl.NumberFormat(undefined, { maximumFractionDigits: 1 }).format(value);
}

const StatsBar: React.FC<StatsBarProps> = ({ stats }) => {
  const cards = [
    { label: 'Words dictated', value: formatNumber(stats.totalWords) },
    { label: 'Minutes saved', value: formatDecimal(stats.minutesSaved) },
    { label: 'Voice WPM', value: formatDecimal(stats.wpm) },
  ];

  return (
    <div className="dashboard-stats-grid">
      {cards.map((card) => (
        <div key={card.label} className="dashboard-stat-card">
          <div className="dashboard-stat-label">{card.label}</div>
          <div className="dashboard-stat-value">{card.value}</div>
        </div>
      ))}
    </div>
  );
};

export default StatsBar;
