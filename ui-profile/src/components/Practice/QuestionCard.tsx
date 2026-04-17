import React from 'react';
import type { PracticeQuestion } from './types';

interface Props {
  question: PracticeQuestion;
  index: number;
  isActive: boolean;
}

export default function QuestionCard({ question, index, isActive }: Props) {
  return (
    <div className={`p-4 rounded-lg border transition ${
      isActive ? 'bg-yellow-500/10 border-yellow-500/40' : 'bg-gray-800/30 border-gray-700/50'
    }`}>
      <div className="flex items-start justify-between mb-2">
        <span className="text-xs text-white/30">Q{index + 1}</span>
        {isActive && <span className="text-xs text-yellow-400">Current</span>}
      </div>
      <p className="text-sm text-white font-medium mb-2">{question.question}</p>
      <p className="text-xs text-yellow-400/70">💡 {question.tips}</p>
    </div>
  );
}