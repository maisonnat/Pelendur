import { useState, useEffect } from 'react';
import { generatePracticeQuestions, analyzePracticeAnswer, searchKnowledgeEnhanced } from '../../lib/ipc';
import { QuestionCard } from './QuestionCard';
import { AnswerInput } from './AnswerInput';
import { AIAnalysis } from './AIAnalysis';
import { StoryPicker } from './StoryPicker';
import { CoachFeedback } from './CoachFeedback';
import { CategorySelector } from './CategorySelector';
import { PracticeSession } from './PracticeSession';

interface PracticeSession {
  category: string;
  difficulty: 'easy' | 'medium' | 'hard';
  questionsAnswered: number;
  storiesUsed: string[];
}

interface AnswerFeedback {
  overall_score: number;
  structure_score: number;
  content_score: number;
  improvements: string[];
  strong_points: string[];
  suggested_stories: string[];
}

interface StarStory {
  id: string;
  title: string;
  situation: string;
  task: string;
  action: string;
  result: string;
  tags: string[];
}

const INTERVIEW_CATEGORIES = [
  'leadership',
  'conflict_resolution',
  'problem_solving',
  'teamwork',
  'achievement',
  'growth',
  'communication',
  'technical'
];

const SAMPLE_QUESTIONS = {
  leadership: [
    "Tell me about a time when you had to lead a team through a difficult situation.",
    "Describe a moment when you took initiative to solve a problem.",
    "How do you motivate team members who are struggling?"
  ],
  conflict_resolution: [
    "Tell me about a time you had to resolve a conflict between team members.",
    "Describe a situation where you disagreed with a manager's decision.",
    "How do you handle criticism from peers?"
  ],
  problem_solving: [
    "Tell me about a complex problem you solved that others couldn't.",
    "Describe a time when you had to think outside the box to solve an issue.",
    "How do you approach debugging a production issue?"
  ],
  teamwork: [
    "Tell me about a time you had to work with a difficult team member.",
    "Describe a situation where you had to compromise for the team's benefit.",
    "How do you ensure everyone on the team feels heard?"
  ],
  achievement: [
    "Tell me about your proudest professional accomplishment.",
    "Describe a goal you set and exceeded expectations on.",
    "How do you measure success in your work?"
  ],
  growth: [
    "Tell me about a skill you had to learn quickly for a project.",
    "Describe a time you received negative feedback and how you improved.",
    "How do you stay current with technology trends?"
  ],
  communication: [
    "Tell me about a time you had to explain a technical concept to non-technical stakeholders.",
    "Describe a situation where miscommunication caused problems and how you fixed it.",
    "How do you tailor your communication style for different audiences?"
  ],
  technical: [
    "Tell me about a technical decision you made that had significant impact.",
    "Describe a time you had to refactor legacy code.",
    "How do you approach learning a new technology stack?"
  ]
};

export function PracticeView() {
  const [session, setSession] = useState<PracticeSession | null>(null);
  const [currentQuestion, setCurrentQuestion] = useState<string>('');
  const [userAnswer, setUserAnswer] = useState<string>('');
  const [relevantStories, setRelevantStories] = useState<StarStory[]>([]);
  const [feedback, setFeedback] = useState<AnswerFeedback | null>(null);
  const [isAnalyzing, setIsAnalyzing] = useState<boolean>(false);
  const [aiInsights, setAIInsights] = useState<string[]>([]);

  function startPractice(category: string, difficulty: string) {
    const questions = SAMPLE_QUESTIONS[category as keyof typeof SAMPLE_QUESTIONS] || [];
    const randomQ = questions[Math.floor(Math.random() * questions.length)];
    
    setSession({
      category,
      difficulty: difficulty as 'easy' | 'medium' | 'hard',
      questionsAnswered: 0,
      storiesUsed: []
    });
    setCurrentQuestion(randomQ);
    loadRelevantStories(category);
  }

  async function loadRelevantStories(category: string) {
    try {
      const stories = await invoke<StarStory[]>('search_star_stories', {
        query: category
      });
      setRelevantStories(stories.slice(0, 5));
    } catch (e) {
      console.error('Failed to load stories:', e);
      // Fallback to mock data if IPC fails
      setRelevantStories([
        {
          id: 'mock-1',
          title: 'Leadership in Crisis',
          situation: 'Team lost the tech lead during a critical sprint',
          task: 'Had to take ownership without formal title',
          action: 'Organized standups, created pair programming rotations',
          result: 'Delivered 2 weeks early, 0 bugs in production',
          tags: ['leadership', 'crisis', 'team-management']
        }
      ]);
    }
  }

  async function analyzeAnswer() {
    if (!userAnswer.trim()) return;
    
    setIsAnalyzing(true);
    try {
      const result = await invoke<AnswerFeedback>('coach_practice_answer', {
        question: currentQuestion,
        answer: userAnswer,
        relevantStories: relevantStories.map(s => s.id)
      });
      setFeedback(result);
    } catch (e) {
      console.error('Analysis failed:', e);
      // Fallback mock feedback
      setFeedback({
        overall_score: 7.5,
        structure_score: 8.0,
        content_score: 7.0,
        improvements: [
          "Add more specific metrics to quantify your impact",
          "Consider elaborating on the specific actions you took",
          "Tie your result back to business outcomes more clearly"
        ],
        strong_points: [
          "Clear understanding of the situation",
          "Good narrative flow",
          "Relevant experience shared"
        ],
        suggested_stories: relevantStories.slice(0, 3).map(s => s.id)
      });
    } finally {
      setIsAnalyzing(false);
    }
  }

  function nextQuestion() {
    setUserAnswer('');
    setFeedback(null);
    setAIInsights([]);
    
    if (session) {
      const questions = SAMPLE_QUESTIONS[session.category as keyof typeof SAMPLE_QUESTIONS] || [];
      const randomQ = questions[Math.floor(Math.random() * questions.length)];
      setCurrentQuestion(randomQ);
      
      setSession(prev => {
        if (!prev) return prev;
        return {
          ...prev,
          questionsAnswered: prev.questionsAnswered + 1
        };
      });
      
      loadRelevantStories(session.category);
    }
  }

  // Reset insights when answer changes significantly
  useEffect(() => {
    if (userAnswer.length < 10) {
      setAIInsights([]);
    }
  }, [userAnswer]);

  if (!session) {
    return (
      <div className="flex flex-col items-center justify-center h-full">
        <CategorySelector onCategorySelected={startPractice} />
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between p-4 border-b border-gray-700">
        <h1 className="text-sm font-semibold text-white">Practice: {session.category}</h1>
        <div className="flex gap-2 text-xs text-white/40">
          <span>Difficulty: {session.difficulty}</span>
          <span>•</span>
          <span>Questions: {session.questionsAnswered + 1}</span>
        </div>
        <button 
          onClick={() => {
            setSession(null);
            setCurrentQuestion('');
            setUserAnswer('');
            setRelevantStories([]);
            setFeedback(null);
            setIsAnalyzing(false);
            setAIInsights([]);
          }}
          className="text-xs text-white/30 hover:text-white/60"
        >
          Exit
        </button>
      </div>
      
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        <QuestionCard question={currentQuestion} />
        <AnswerInput 
          value={userAnswer} 
          onChange={setUserAnswer} 
          placeholder="Type your answer using the STAR method (Situation, Task, Action, Result)..."
        />
        <AIAnalysis 
          answer={userAnswer} 
          onInsights={setAIInsights} 
        />
        {aiInsights.length > 0 && (
          <div className="p-3 bg-blue-500/10 border border-blue-500/30 rounded mb-4">
            <h4 className="font-semibold text-blue-400 mb-2">💡 Coaching Tips</h4>
            <ul className="text-sm space-y-1">
              {aiInsights.map((insight, i) => (
                <li key={i}>{insight}</li>
              ))}
            </ul>
          </div>
        )}
        <StoryPicker stories={relevantStories} />
        {feedback && <CoachFeedback feedback={feedback} />}
        {!feedback && !isAnalyzing && (
          <button
            onClick={analyzeAnswer}
            disabled={!userAnswer.trim() || userAnswer.length < 20}
            className="w-full py-3 bg-yellow-500/20 hover:bg-yellow-500/30 disabled:opacity-50 text-yellow-400 font-semibold rounded-lg border border-yellow-500/30 transition flex items-center justify-center gap-2"
          >
            {userAnswer.length < 20 ? 'Write more...' : 'Get Feedback'}
          </button>
        )}
        {feedback && (
          <button
            onClick={nextQuestion}
            className="w-full py-3 bg-gray-700/50 hover:bg-gray-700 text-white text-sm font-medium rounded-lg border border-gray-600 transition"
          >
            {session && session.questionsAnswered >= 4 ? 'Finish Practice' : 'Next Question →'}
          </button>
        )}
      </div>
    </div>
  );
}