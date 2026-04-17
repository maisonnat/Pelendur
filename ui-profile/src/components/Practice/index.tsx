import React, { useState } from 'react';
import { generatePracticeQuestions, analyzePracticeAnswer } from '../../lib/ipc';
import type { PracticeQuestion, AnswerFeedback, PracticeMode } from './types';
import ModeSelector from './ModeSelector';
import QuestionCard from './QuestionCard';
import FeedbackPanel from './FeedbackPanel';

export default function Practice() {
  const [mode, setMode] = useState<PracticeMode>('behavioral');
  const [questions, setQuestions] = useState<PracticeQuestion[]>([]);
  const [currentIdx, setCurrentIdx] = useState(0);
  const [answer, setAnswer] = useState('');
  const [feedback, setFeedback] = useState<AnswerFeedback | null>(null);
  const [loading, setLoading] = useState(false);
  const [analyzing, setAnalyzing] = useState(false);
  const [started, setStarted] = useState(false);
  const [companyName, setCompanyName] = useState('');

  async function startPractice() {
    setLoading(true);
    setFeedback(null);
    setAnswer('');
    try {
      const qs = await generatePracticeQuestions(mode, mode === 'company' ? companyName || undefined : undefined);
      setQuestions(qs);
      setCurrentIdx(0);
      setStarted(true);
    } catch (e) {
      console.error(e);
    } finally {
      setLoading(false);
    }
  }

  async function submitAnswer() {
    if (!answer.trim()) return;
    setAnalyzing(true);
    try {
      const fb = await analyzePracticeAnswer(questions[currentIdx].question, answer, mode);
      setFeedback(fb);
    } catch (e) {
      console.error(e);
    } finally {
      setAnalyzing(false);
    }
  }

  function nextQuestion() {
    setAnswer('');
    setFeedback(null);
    if (currentIdx < questions.length - 1) {
      setCurrentIdx(currentIdx + 1);
    } else {
      setStarted(false);
    }
  }

  if (!started) {
    return (
      <div className="flex flex-col items-center justify-center h-full max-w-lg mx-auto gap-6">
        <div className="text-center mb-4">
          <div className="text-4xl mb-2">🎯</div>
          <h1 className="text-xl font-bold text-white">Interview Practice</h1>
          <p className="text-sm text-white/50 mt-1">Practice with AI-generated questions and real-time feedback</p>
        </div>
        <ModeSelector selected={mode} onSelect={setMode} />
        {mode === 'company' && (
          <input
            type="text"
            placeholder="Company name..."
            value={companyName}
            onChange={e => setCompanyName(e.target.value)}
            className="w-full px-4 py-2 bg-gray-800/50 border border-gray-700 rounded-lg text-sm text-white placeholder-white/20 focus:outline-none focus:border-yellow-500/50"
          />
        )}
        <button
          onClick={startPractice}
          disabled={loading}
          className="w-full py-3 bg-yellow-500/20 hover:bg-yellow-500/30 disabled:opacity-50 text-yellow-400 font-semibold rounded-lg border border-yellow-500/30 transition flex items-center justify-center gap-2"
        >
          {loading ? <div className="w-5 h-5 border-2 border-yellow-500/30 border-t-yellow-500 rounded-full animate-spin" /> : '🚀'} 
          {loading ? 'Generating...' : 'Start Practice'}
        </button>
      </div>
    );
  }

  const current = questions[currentIdx];

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between p-4 border-b border-gray-700">
        <h1 className="text-sm font-semibold text-white">Practice: {mode.charAt(0).toUpperCase() + mode.slice(1)} Mode</h1>
        <div className="flex gap-2">
          <span className="text-xs text-white/40">{currentIdx + 1}/{questions.length}</span>
          <button onClick={() => { setStarted(false); setQuestions([]); }} className="text-xs text-white/30 hover:text-white/60">Exit</button>
        </div>
      </div>
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        <QuestionCard question={current} index={currentIdx} isActive={true} />
        <div>
          <textarea
            value={answer}
            onChange={e => setAnswer(e.target.value)}
            placeholder="Type or dictate your answer..."
            className="w-full h-32 p-3 bg-gray-800/50 border border-gray-700 rounded-lg text-sm text-white placeholder-white/20 resize-none focus:outline-none focus:border-yellow-500/50"
          />
          <div className="flex gap-2 mt-2">
            {!feedback ? (
              <button
                onClick={submitAnswer}
                disabled={analyzing || !answer.trim()}
                className="flex-1 py-2 bg-yellow-500/20 hover:bg-yellow-500/30 disabled:opacity-50 text-yellow-400 text-sm font-medium rounded-lg border border-yellow-500/30 transition flex items-center justify-center gap-2"
              >
                {analyzing && <div className="w-4 h-4 border-2 border-yellow-500/30 border-t-yellow-500 rounded-full animate-spin" />}
                {analyzing ? 'Analyzing...' : 'Submit Answer'}
              </button>
            ) : (
              <button
                onClick={nextQuestion}
                className="flex-1 py-2 bg-gray-700/50 hover:bg-gray-700 text-white text-sm font-medium rounded-lg border border-gray-600 transition"
              >
                {currentIdx < questions.length - 1 ? 'Next Question →' : 'Finish Practice'}
              </button>
            )}
          </div>
        </div>
        {feedback && <FeedbackPanel feedback={feedback} />}
      </div>
    </div>
  );
}