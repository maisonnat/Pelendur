import React from 'react';
import { SuggestionCard } from './SuggestionCard';

interface SuggestionListProps {
  suggestions: any[];
  approvedSuggestions: Set<string>;
  onToggleApproval: (suggestionId: string) => void;
}

export function SuggestionList({ 
  suggestions, 
  approvedSuggestions, 
  onToggleApproval 
}: SuggestionListProps) {
  if (suggestions.length === 0) {
    return null;
  }

  // Group suggestions by type
  const grouped = suggestions.reduce((acc, suggestion) => {
    const type = suggestion.suggestion_type;
    if (!acc[type]) {
      acc[type] = [];
    }
    acc[type].push(suggestion);
    return acc;
  }, {} as Record<string, any[]>);

  const typeLabels: Record<string, string> = {
    SkillMentioned: 'Habilidades Detectadas',
    PotentialStarStory: 'Historias STAR Potenciales',
    ImprovementArea: 'Áreas de Mejora',
    StrongAnswer: 'Respuestas Fuertes'
  };

  return (
    <div className="space-y-4">
      {Object.entries(grouped).map(([type, suggestions]) => (
        <div key={type}>
          <h2 className="text-sm font-semibold text-white/60 uppercase tracking-wider mb-3">
            {typeLabels[type] || type} ({suggestions.length})
          </h2>
          <div className="space-y-3">
            {suggestions.map((suggestion) => (
              <SuggestionCard 
                key={suggestion.id} 
                suggestion={suggestion} 
                isApproved={approvedSuggestions.has(suggestion.id)} 
                onToggle={() => onToggleApproval(suggestion.id)} 
              />
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
