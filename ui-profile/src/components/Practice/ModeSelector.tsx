import React from 'react';
import type { PracticeMode } from './types';

interface Props {
  selected: PracticeMode;
  onSelect: (m: PracticeMode) => void;
}

export default function ModeSelector({ selected, onSelect }: Props) {
  const modes = [
    { key: 'behavioral' as PracticeMode, icon: '💬', label: 'Behavioral', desc: 'STAR-structured questions about past experiences' },
    { key: 'technical' as PracticeMode, icon: '🛠️', label: 'Technical', desc: 'Domain-specific questions from your skills' },
    { key: 'company' as PracticeMode, icon: '🏢', label: 'Company', desc: 'Tailored to a specific company culture' },
  ];
  return (
    <div className="grid grid-cols-3 gap-3 mb-6">
      {modes.map(m => (
        <button
          key={m.key}
          onClick={() => onSelect(m.key)}
          className={`p-4 rounded-lg border text-left transition ${
            selected === m.key
              ? 'bg-yellow-500/20 border-yellow-500/50'
              : 'bg-gray-800/50 border-gray-700 hover:border-gray-500'
          }`}
        >
          <div className="text-2xl mb-1">{m.icon}</div>
          <div className={`text-sm font-semibold ${selected === m.key ? 'text-yellow-400' : 'text-white'}`}>{m.label}</div>
          <div className="text-xs text-white/40 mt-1">{m.desc}</div>
        </button>
      ))}
    </div>
  );
}