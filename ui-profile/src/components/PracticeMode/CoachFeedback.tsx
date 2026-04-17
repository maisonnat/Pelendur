import { FC } from 'react';

interface AnswerFeedback {
  overall_score: number;
  structure_score: number;
  content_score: number;
  improvements: string[];
  strong_points: string[];
  suggested_stories: string[];
}

interface CoachFeedbackProps {
  feedback: AnswerFeedback;
}

export const CoachFeedback: FC<CoachFeedbackProps> = ({ feedback }) => {
  return (
    <div className="space-y-4">
      {/* Overall Score */}
      <div className="flex items-center gap-4">
        <span className="text-3xl font-bold text-yellow-500">
          {feedback.overall_score}/10
        </span>
        <div className="flex-1">
          <div className="h-2 bg-gray-700 rounded">
            <div 
              className="h-2 bg-yellow-500 rounded transition-all"
              style={{ width: `${feedback.overall_score * 10}%` }}
            />
          </div>
        </div>
      </div>
      
      {/* Scores Breakdown */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <span className="text-sm text-gray-400">Structure</span>
          <div className="text-lg font-semibold">{feedback.structure_score}/10</div>
        </div>
        <div>
          <span className="text-sm text-gray-400">Content</span>
          <div className="text-lg font-semibold">{feedback.content_score}/10</div>
        </div>
      </div>
      
      {/* Strong Points */}
      {feedback.strong_points.length > 0 && (
        <div className="p-3 bg-green-500/10 border border-green-500/30 rounded">
          <h4 className="font-semibold text-green-400 mb-2">✓ Strengths</h4>
          <ul className="text-sm space-y-1">
            {feedback.strong_points.map((point, i) => (
              <li key={i}>{point}</li>
            ))}
          </ul>
        </div>
      )}
      
      {/* Improvements */}
      {feedback.improvements.length > 0 && (
        <div className="p-3 bg-orange-500/10 border border-orange-500/30 rounded">
          <h4 className="font-semibold text-orange-400 mb-2">→ Areas for Improvement</h4>
          <ul className="text-sm space-y-1">
            {feedback.improvements.map((imp, i) => (
              <li key={i}>{imp}</li>
            ))}
          </ul>
        </div>
      )}
      
      {/* Suggested Stories */}
      {feedback.suggested_stories.length > 0 && (
        <div className="p-3 bg-blue-500/10 border border-blue-500/30 rounded">
          <h4 className="font-semibold text-blue-400 mb-2">📖 Suggested Stories to Reference</h4>
          <p className="text-sm text-white/70">
            Consider adapting these STAR stories to strengthen your answer:
          </p>
          <ul className="text-sm space-y-1 mt-1">
            {feedback.suggested_stories.map((storyId, i) => (
              <li key={i}>Story #{storyId.substring(0, 8)}...</li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
};