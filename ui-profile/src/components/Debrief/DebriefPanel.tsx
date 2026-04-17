import { useState, useEffect } from 'react';
import { analyzeMeeting } from '../../lib/ipc';
import type { MeetingAnalysis, MeetingSuggestion } from './types';
import { SuggestionCard } from './SuggestionCard';
import { MeetingSummary } from './MeetingSummary';
import { StrongMoments } from './StrongMoments';
import { ImprovementAreas } from './ImprovementAreas';
import { SuggestionList } from './SuggestionList';

interface LearningSuggestion {
  id: string;
  suggestion_type: 'SkillMentioned' | 'PotentialStarStory' | 'ImprovementArea' | 'StrongAnswer';
  title: string;
  description: string;
  confidence: number;
  source_excerpt: string;
}

interface MeetingAnalysis {
  meeting_id: string;
  transcript: string;
  suggestions: LearningSuggestion[];
  summary: string;
  duration_minutes: number;
}

interface DebriefProps {
  meetingId: string;
  transcript: string;
  durationMinutes: number;
  onClose: () => void;
  onSave: (approvedSuggestions: LearningSuggestion[]) => void;
}

export function DebriefPanel({ 
  meetingId, 
  transcript,
  durationMinutes,
  onClose,
  onSave 
}: DebriefProps) {
  const [analysis, setAnalysis] = useState<MeetingAnalysis | null>(null);
  const [loading, setLoading] = useState(true);
  const [approvedSuggestions, setApprovedSuggestions] = useState<Set<string>>(new Set());

  useEffect(() => {
    loadAnalysis();
  }, [meetingId, transcript, durationMinutes]);

  async function loadAnalysis() {
    try {
      const result = await analyzeMeeting(transcript);
      setAnalysis({
        ...result,
        meeting_id: meetingId,
        transcript,
        duration_minutes: durationMinutes,
      } as MeetingAnalysis);
    } catch (e) {
      console.error('Failed to analyze meeting:', e);
    } finally {
      setLoading(false);
    }
  }

  function toggleApproval(suggestionId: string) {
    setApprovedSuggestions(prev => {
      const next = new Set(prev);
      if (next.has(suggestionId)) {
        next.delete(suggestionId);
      } else {
        next.add(suggestionId);
      }
      return next;
    });
  }

  async function handleSave() {
    const approved = analysis?.suggestions.filter(s => approvedSuggestions.has(s.id)) ?? [];
    await onSave(approved);
    onClose();
  }

  if (loading) {
    return (
      <div className="flex flex-col items-center justify-center h-full">
        <div className="w-12 h-12 border-2 border-yellow-500/30 border-t-yellow-500 rounded-full animate-spin" />
        <p className="mt-2 text-white/50 text-sm">Analyzing meeting...</p>
      </div>
    );
  }

  if (!analysis) {
    return (
      <div className="flex flex-col h-full">
        <div className="flex items-center justify-between p-4 border-b border-gray-700">
          <h1 className="text-lg font-semibold text-white">📋 Meeting Debrief</h1>
          <button onClick={onClose} className="text-white/40 hover:text-white text-xl">×</button>
        </div>

        <div className="flex-1 overflow-y-auto p-4">
          <div className="text-center py-8">
            <p className="text-white/40 text-sm">No analysis available. Please check the meeting transcript.</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full bg-gray-900/50 backdrop-blur-sm">
      <div className="flex items-center justify-between p-4 border-b border-gray-700">
        <h1 className="text-lg font-semibold text-white">📋 Meeting Debrief</h1>
        <button onClick={onClose} className="text-white/40 hover:text-white text-xl">×</button>
      </div>

      <div className="flex-1 overflow-y-auto p-4 space-y-6">
        <MeetingSummary summary={analysis.summary} durationMinutes={analysis.duration_minutes} />
        
        <div className="space-y-4">
          <SuggestionList 
            suggestions={analysis.suggestions} 
            approvedSuggestions={approvedSuggestions} 
            onToggleApproval={toggleApproval} 
          />
          
          <StrongMoments 
            suggestions={analysis.suggestions.filter(s => s.suggestion_type === 'StrongAnswer')} 
          />
          
          <ImprovementAreas 
            suggestions={analysis.suggestions.filter(s => s.suggestion_type === 'ImprovementArea')} 
          />
        </div>

        <div className="mt-6 pt-4 border-t border-gray-700">
          <button
            onClick={handleSave}
            className="w-full py-3 bg-yellow-500/20 hover:bg-yellow-500/30 text-yellow-400 font-semibold rounded-lg border border-yellow-500/30 transition flex items-center justify-center gap-2"
          >
            <span>💾 Guardar</span>
            <span className="text-xs opacity-70">{approvedSuggestions.size} aprobadas</span>
          </button>
        </div>
      </div>
    </div>
  );
}
