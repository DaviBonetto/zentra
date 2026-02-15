import React from 'react';
import type { HistoryItem as HistoryEntry } from '../types';
import HistoryItem from './HistoryItem';

interface HistoryProps {
  items: HistoryEntry[];
  onDelete: (id: string) => void;
  onCopied: () => void;
}

const History: React.FC<HistoryProps> = ({ items, onDelete, onCopied }) => {
  if (items.length === 0) {
    return (
      <div className="dashboard-empty-state">
        <div className="dashboard-empty-icon">🎙</div>
        <h3>No transcriptions yet</h3>
        <p>Press Ctrl+Shift+Space to start dictating.</p>
      </div>
    );
  }

  return (
    <div className="dashboard-history-list">
      {items.map((item) => (
        <HistoryItem key={item.id} item={item} onDelete={onDelete} onCopied={onCopied} />
      ))}
    </div>
  );
};

export default History;
