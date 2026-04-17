import React from 'react';

interface ImprovementAreasProps {
  suggestions: any[];
}

export function ImprovementAreas({ suggestions }: ImprovementAreasProps) {
  if (suggestions.length === 0) {
    return null;
  }

  return (
    <div className="bg-orange-900/20 border border-orange-700/30 rounded-lg p-4">
      <div className="flex items-center gap-3 mb-3">
        <span className="text-2xl">📝</span>
        <h2 className="text-lg font-semibold text-white">Áreas de Mejora</h2>
      </div>
      <p className="text-sm text-orange-400 mb-3">
        Estas son las áreas donde puedes mejorar en futuras reuniones:
      </p>
      <div className="space-y-3">
        {suggestions.map((suggestion, index) => (
          <div key={index} className="bg-orange-800/30 p-3 rounded-lg">
            <h3 className="font-semibold text-orange-300">{suggestion.title}</h3>
            <p className="text-sm text-orange-200">{suggestion.description}</p>
            <blockquote className="mt-2 text-xs text-orange-100 border-l-2 pl-2 border-orange-500/30">
              "{suggestion.source_excerpt}"
            </blockquote>
          </div>
        ))}
      </div>
    </div>
  );
}
