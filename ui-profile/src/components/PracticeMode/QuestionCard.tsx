import { FC } from 'react';

interface QuestionCardProps {
  question: string;
}

export const QuestionCard: FC<QuestionCardProps> = ({ question }) => {
  return (
    <div className="bg-[rgba(255,215,0,0.03)] border border-[rgba(255,215,0,0.08)] rounded-xl px-6 py-5">
      <p className="text-[10px] text-[#ffd700]/30 uppercase tracking-wider mb-2">
        Interview Question
      </p>
      <p className="text-base text-white/70 leading-relaxed">
        "{question}"
      </p>
    </div>
  );
};