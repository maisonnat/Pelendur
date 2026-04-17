import { useState } from 'react';

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

const DIFFICULTY_LEVELS = [
  { value: 'easy', label: 'Easy', color: 'bg-green-500/20 text-green-400' },
  { value: 'medium', label: 'Medium', color: 'bg-yellow-500/20 text-yellow-400' },
  { value: 'hard', label: 'Hard', color: 'bg-red-500/20 text-red-400' }
];

interface CategorySelectorProps {
  onCategorySelected: (category: string, difficulty: string) => void;
}

export function CategorySelector({ onCategorySelected }: CategorySelectorProps) {
  const [selectedCategory, setSelectedCategory] = useState<string>('');
  const [selectedDifficulty, setSelectedDifficulty] = useState<string>('medium');

  const handleStartPractice = () => {
    if (selectedCategory && selectedDifficulty) {
      onCategorySelected(selectedCategory, selectedDifficulty);
    }
  };

  return (
    <div className="flex flex-col items-center justify-center h-full space-y-6">
      <div className="text-center">
        <div className="text-5xl mb-4">🎯</div>
        <h1 className="text-2xl font-bold text-white">Interview Practice Mode</h1>
        <p className="text-sm text-white/50 max-w-md">
          Simulate interview questions and get AI-powered feedback on your STAR stories
        </p>
      </div>
      
      <div className="w-full max-w-md space-y-4">
        <div className="space-y-3">
          <label className="flex items-center justify-between text-sm text-white/70">
            <span>Interview Category</span>
            <span className="text-xs text-white/40">
              {selectedCategory || 'Select category'}
            </span>
          </label>
          <div className="space-y-2">
            {INTERVIEW_CATEGORIES.map((category) => (
              <button
                key={category}
                onClick={() => setSelectedCategory(category)}
                className={`w-full flex items-center justify-between px-4 py-3 bg-gray-800/30 border border-gray-700 rounded-lg hover:bg-gray-800/40 transition-colors ${
                  selectedCategory === category ? 'bg-yellow-500/20 border-yellow-500/30' : ''
                }`}
              >
                <span className="text-white/80 capitalize">
                  {category.replace('_', ' ')}
                </span>
                <span className="text-xs">
                  {selectedCategory === category ? '✓' : ''}
                </span>
              </button>
            ))}
          </div>
        </div>
        
        <div className="space-y-3">
          <label className="flex items-center justify-between text-sm text-white/70">
            <span>Difficulty Level</span>
            <span className="text-xs text-white/40">
              {selectedDifficulty || 'Select difficulty'}
            </span>
          </label>
          <div className="space-y-2">
            {DIFFICULTY_LEVELS.map((level) => (
              <button
                key={level.value}
                onClick={() => setSelectedDifficulty(level.value)}
                className={`w-full flex items-center justify-between px-4 py-3 bg-gray-800/30 border border-gray-700 rounded-lg hover:bg-gray-800/40 transition-colors ${
                  selectedDifficulty === level.value ? level.color.replace('/20', '/30').replace('/40', '/30') : ''
                }`}
              >
                <span className="flex items-center gap-2">
                  <span className={`${level.color} font-medium`}>
                    {level.label}
                  </span>
                </span>
                <span className="text-xs">
                  {selectedDifficulty === level.value ? '✓' : ''}
                </span>
              </button>
            ))}
          </div>
        </div>
      </div>
      
      <button
        onClick={handleStartPractice}
        disabled={!selectedCategory || !selectedDifficulty}
        className="w-full py-4 bg-yellow-500/20 hover:bg-yellow-500/30 disabled:opacity-50 text-yellow-400 font-semibold rounded-lg border border-yellow-500/30 transition flex items-center justify-center gap-2"
      >
        {(!selectedCategory || !selectedDifficulty) ? 'Select category and difficulty to begin' : '🚀 Start Practice Session'}
      </button>
    </div>
  );
}