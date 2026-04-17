import { FC } from 'react';

interface PracticeSessionProps {
  category: string;
  difficulty: 'easy' | 'medium' | 'hard';
  questionsAnswered: number;
  onReset: () => void;
}

export const PracticeSession: FC<PracticeSessionProps> = ({ 
  category, 
  difficulty, 
  questionsAnswered,
  onReset 
}) => {
  return (
    <div className="flex items-center justify-between p-3 bg-gray-800/30 border border-gray-700 rounded-lg">
      <div className="flex items-center gap-3 text-sm">
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 bg-yellow-400 rounded" />
          <span className="text-white/60">{category}</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 bg-blue-400 rounded" />
          <span className="text-white/60">{difficulty}</span>
        </div>
      </div>
      <div className="flex items-center gap-3 text-xs text-white/40">
        <span>Questions: {questionsAnswered}</span>
        <span>•</span>
        <span>Session Active</span>
        <button 
          onClick={onReset}
          className="p-1 hover:text-white/60 transition-colors"
          title="Reset session"
        >
          ⟳
        </button>
      </div>
    </div>
  );
};