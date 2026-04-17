export interface PracticeQuestion {
  question_type: string;
  question: string;
  tips: string;
  expected_aspects: string[];
}

export interface AnswerFeedback {
  structure_score: number;
  specificity_score: number;
  relevance_score: number;
  overall_score: number;
  feedback: string;
  improvements: string[];
  strong_points: string[];
}

export type PracticeMode = 'behavioral' | 'technical' | 'company';