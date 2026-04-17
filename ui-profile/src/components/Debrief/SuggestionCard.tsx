import React from 'react';
import type { MeetingSuggestion } from './types';

interface SuggestionCardProps {
  suggestion: MeetingSuggestion;
  isApproved: boolean;
  onToggle: () => void;
}

export function SuggestionCard({ suggestion, isApproved, onToggle }: SuggestionCardProps) {
  const typeIcons = {
    skill: '🎯',
    star_story: '⭐',
    improvement: '📝',
    strong_answer: '✨'
  };

  const typeColors = {
    skill: 'border-blue-500 bg-blue-500/10',
    star_story: 'border-yellow-500 bg-yellow-500/10',
    improvement: 'border-orange-500 bg-orange-500/10',
    strong_answer: 'border-green-500 bg-green-500/10'
  };

  return (
    <div className={`border rounded-lg p-4 ${typeColors[suggestion.suggestion_type]}`}>
      <div className="flex items-start justify-between">
        <div className="flex items-center gap-2">
          <span className="text-xl">{typeIcons[suggestion.suggestion_type]}</span>
          <span className="font-semibold">{suggestion.title}</span>
          <span className="text-xs opacity-60">
            {Math.round(suggestion.confidence * 100)}% confidence
          </span>
        </div>
        <button
          onClick={onToggle}
          className={`px-3 py-1 rounded ${isApproved 
            ? 'bg-green-600 text-white' 
            : 'bg-gray-700 text-gray-300'}`}
        >
          {isApproved ? '✓ Aprobado' : 'Aprobar'}
        </button>
      </div>
      <p className="mt-2 text-sm text-gray-300">{suggestion.description}</p>
      <blockquote className="mt-2 text-xs text-gray-500 border-l-2 pl-2">
        "{suggestion.source_excerpt}"
      </blockquote>
    </div>
  );
}
