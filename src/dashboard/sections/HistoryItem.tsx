import React, { useMemo, useState } from 'react';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import type { HistoryItem as HistoryEntry } from '../types';

interface HistoryItemProps {
  item: HistoryEntry;
  onDelete: (id: string) => void;
  onCopied: () => void;
}

const PREVIEW_LIMIT = 120;

const HistoryItem: React.FC<HistoryItemProps> = ({ item, onDelete, onCopied }) => {
  const [expanded, setExpanded] = useState(false);

  const previewText = useMemo(() => {
    if (item.text.length <= PREVIEW_LIMIT) return item.text;
    if (expanded) return item.text;
    return `${item.text.slice(0, PREVIEW_LIMIT)}...`;
  }, [expanded, item.text]);

  const timestamp = useMemo(() => {
    const parsed = new Date(item.timestamp);
    if (Number.isNaN(parsed.getTime())) return item.timestamp;
    return parsed.toLocaleString();
  }, [item.timestamp]);

  return (
    <div className="dashboard-history-item">
      <div className="dashboard-history-icon">≋</div>
      <div className="dashboard-history-main">
        <p className="dashboard-history-text">{previewText}</p>
        {item.text.length > PREVIEW_LIMIT && (
          <button type="button" className="dashboard-inline-link" onClick={() => setExpanded((prev) => !prev)}>
            {expanded ? 'Show less' : 'Show more'}
          </button>
        )}
      </div>
      <div className="dashboard-history-meta">
        <span>{timestamp}</span>
        <span>{item.wordCount} words</span>
        <div className="dashboard-history-actions">
          <button
            type="button"
            onClick={async () => {
              await writeText(item.text);
              onCopied();
            }}
          >
            Copy
          </button>
          <button type="button" onClick={() => onDelete(item.id)}>
            Delete
          </button>
        </div>
      </div>
    </div>
  );
};

export default HistoryItem;
