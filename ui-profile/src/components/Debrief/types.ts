export interface MeetingSuggestion {
  suggestion_type: 'skill' | 'star_story' | 'improvement' | 'strong_answer';
  title: string;
  description: string;
  confidence: number;
  data: Record<string, unknown>;
}

export interface MeetingAnalysis {
  suggestions: MeetingSuggestion[];
  summary: string;
  skills_found: string[];
}
