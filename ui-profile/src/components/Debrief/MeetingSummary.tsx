import React from 'react';

interface MeetingSummaryProps {
  summary: string;
  durationMinutes: number;
}

export function MeetingSummary({ summary, durationMinutes }: MeetingSummaryProps) {
  return (
    <div className="bg-gray-800/50 border border-gray-700 rounded-lg p-4">
      <div className="flex items-center gap-3 mb-2">
        <span className="text-2xl">📝</span>
        <h2 className="text-lg font-semibold text-white">Resumen de la Reunión</h2>
      </div>
      <p className="text-sm text-gray-300">{durationMinutes} minutos</p>
      <p className="mt-2 text-white">{summary}</p>
    </div>
  );
}
