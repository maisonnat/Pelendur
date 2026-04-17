import React from 'react';
import type { AnswerFeedback } from './types';

function ScoreBar({ label, score }: { label: string; score: number }) {
  const pct = Math.round(score * 100);
  const color = pct >= 70 ? 'bg-green-500' : pct >= 40 ? 'bg-yellow-500' : 'bg-red-500';
  return (
    <div className="mb-3">
      <div className="flex justify-between text-xs text-white/60 mb-1">
        <span>{label}</span>
        <span className={pct >= 70 ? 'text-green-400' : pct >= 40 ? 'text-yellow-400' : 'text-red-400'}>{pct}%</span>
      </div>
      <div className="h-1.5 bg-gray-700 rounded-full overflow-hidden">
        <div className={`h-full ${color} rounded-full transition-all`} style={{ width: `${pct}%` }} />
      </div>
    </div>
  );
}

interface Props {
  feedback: AnswerFeedback;
}

export default function FeedbackPanel({ feedback }: Props) {
  const overallColor = feedback.overall_score >= 0.7 ? 'text-green-400' : feedback.overall_score >= 0.4 ? 'text-yellow-400' : 'text-red-400';
  return (
    <div className="space-y-4">
      <div className="text-center">
        <div className={`text-4xl font-bold ${overallColor}`}>{Math.round(feedback.overall_score * 100)}</div>
        <div className="text-xs text-white/40 uppercase tracking-wider">Overall Score</div>
      </div>
      <ScoreBar label="Structure (STAR)" score={feedback.structure_score} />
      <ScoreBar label="Specificity (Metrics)" score={feedback.specificity_score} />
      <ScoreBar label="Relevance" score={feedback.relevance_score} />
      <div className="p-3 bg-gray-800/30 border border-gray-700 rounded-lg">
        <p className="text-sm text-white/80">{feedback.feedback}</p>
      </div>
      {feedback.strong_points.length > 0 && (
        <div>
          <p className="text-xs text-green-400 uppercase tracking-wider mb-2">✓ Strong Points</p>
          {feedback.strong_points.map((p, i) => <p key={i} className="text-xs text-white/60 mb-1">• {p}</p>)}
        </div>
      )}
      {feedback.improvements.length > 0 && (
        <div>
          <p className="text-xs text-yellow-400 uppercase tracking-wider mb-2">→ Improvements</p>
          {feedback.improvements.map((p, i) => <p key={i} className="text-xs text-white/60 mb-1">• {p}</p>)}
        </div>
      )}
    </div>
  );
}