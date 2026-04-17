import React from 'react';

interface StrongMomentsProps {
  suggestions: any[];
}

export function StrongMoments({ suggestions }: StrongMomentsProps) {
  if (suggestions.length === 0) {
    return null;
  }

  return (
    <div className="bg-green-900/20 border border-green-700/30 rounded-lg p-4">
      <div className="flex items-center gap-3 mb-3">
        <span className="text-2xl">✨</span>
        <h2 className="text-lg font-semibold text-white">Momentos Fuertes</h2>
      </div>
      <p className="text-sm text-green-400 mb-3">
        Estas fueron las respuestas más efectivas durante la reunión:
      </p>
      <div className="space-y-3">
        {suggestions.map((suggestion, index) => (
          <div key={index} className="bg-green-800/30 p-3 rounded-lg">
            <h3 className="font-semibold text-green-300">{suggestion.title}</h3>
            <p className="text-sm text-green-200">{suggestion.description}</p>
            <blockquote className="mt-2 text-xs text-green-100 border-l-2 pl-2 border-green-500/30">
              "{suggestion.source_excerpt}"
            </blockquote>
          </div>
        ))}
      </div>
    </div>
  );
}
