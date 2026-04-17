import { useState, useEffect, useCallback, useRef } from "react";
import type { StarStoryRecord } from "../../lib/ipc";

interface PracticeModeProps {
  stories: StarStoryRecord[];
  onClose: () => void;
}

const BEHAVIORAL_QUESTIONS = [
  "Tell me about a time you led a team through a difficult challenge.",
  "Describe a situation where you had to deal with conflicting priorities.",
  "Give me an example of when you had to influence someone without authority.",
  "Tell me about a time you failed and what you learned from it.",
  "Describe your most significant technical achievement.",
  "Tell me about a time you had to make a decision with incomplete information.",
  "Describe a situation where you had to push back on a requirement.",
  "Give me an example of how you handled a conflict with a teammate.",
  "Tell me about a time you improved a process or system.",
  "Describe a situation where you had to learn something new quickly.",
  "Tell me about a time you had to communicate complex technical information to non-technical stakeholders.",
  "Give me an example of when you went above and beyond what was expected.",
];

function parseTags(tags: string | null): string[] {
  if (!tags) return [];
  try {
    return JSON.parse(tags);
  } catch {
    return [];
  }
}

export default function PracticeMode({ stories, onClose }: PracticeModeProps) {
  const [currentQuestion, setCurrentQuestion] = useState("");
  const [selectedStoryId, setSelectedStoryId] = useState<string | null>(null);
  const [phase, setPhase] = useState<"question" | "reveal" | "recap">("question");
  const [timerSeconds, setTimerSeconds] = useState(0);
  const [isTimerRunning, setIsTimerRunning] = useState(false);
  const [practicedIds, setPracticedIds] = useState<Set<string>>(new Set());
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const generateQuestion = useCallback(() => {
    const idx = Math.floor(Math.random() * BEHAVIORAL_QUESTIONS.length);
    setCurrentQuestion(BEHAVIORAL_QUESTIONS[idx]);
    setPhase("question");
    setSelectedStoryId(null);
    setTimerSeconds(0);
    setIsTimerRunning(false);
  }, []);

  useEffect(() => {
    generateQuestion();
  }, [generateQuestion]);

  useEffect(() => {
    if (isTimerRunning) {
      timerRef.current = setInterval(() => {
        setTimerSeconds((s) => s + 1);
      }, 1000);
    }
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [isTimerRunning]);

  const formatTime = (s: number) => {
    const mins = Math.floor(s / 60);
    const secs = s % 60;
    return `${mins}:${secs.toString().padStart(2, "0")}`;
  };

  const handleStartAnswer = () => {
    setIsTimerRunning(true);
  };

  const handleReveal = () => {
    setIsTimerRunning(false);
    setPhase("reveal");
  };

  const handleSelectStory = (id: string) => {
    setSelectedStoryId(id === selectedStoryId ? null : id);
  };

  const handleMarkPracticed = () => {
    if (selectedStoryId) {
      setPracticedIds((prev) => new Set(prev).add(selectedStoryId));
    }
    setPhase("recap");
  };

  const handleNext = () => {
    generateQuestion();
  };

  const selectedStory = stories.find((s) => s.id === selectedStoryId);

  return (
    <div className="fixed inset-0 z-30 bg-[rgba(5,5,5,0.92)] backdrop-blur-xl flex items-center justify-center">
      <div className="w-full max-w-3xl mx-4 bg-[rgba(20,20,20,0.95)] border border-white/[0.06] rounded-2xl shadow-2xl overflow-hidden">
        <div className="flex items-center justify-between px-6 py-4 border-b border-white/[0.04]">
          <div className="flex items-center gap-3">
            <span className="text-[#ffd700] text-lg">✦</span>
            <h2 className="text-sm font-medium text-white/60 uppercase tracking-wider">
              Practice Mode
            </h2>
          </div>
          <div className="flex items-center gap-4">
            <span className="text-xs text-white/20">
              {practicedIds.size}/{stories.length} practiced
            </span>
            <button
              type="button"
              onClick={onClose}
              className="text-white/20 hover:text-white/50 transition-colors text-sm"
            >
              ✕
            </button>
          </div>
        </div>

        <div className="p-6 space-y-5">
          {phase === "question" && (
            <>
              <div className="bg-[rgba(255,215,0,0.03)] border border-[rgba(255,215,0,0.08)] rounded-xl px-6 py-5">
                <p className="text-[10px] text-[#ffd700]/30 uppercase tracking-wider mb-2">
                  Interview Question
                </p>
                <p className="text-base text-white/70 leading-relaxed">
                  "{currentQuestion}"
                </p>
              </div>

              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <span className="text-2xl font-mono text-white/40 tabular-nums">
                    {formatTime(timerSeconds)}
                  </span>
                  {!isTimerRunning && timerSeconds === 0 && (
                    <button
                      type="button"
                      onClick={handleStartAnswer}
                      className="px-3 py-1.5 text-xs rounded-lg bg-[rgba(255,215,0,0.1)] text-[#ffd700] border border-[rgba(255,215,0,0.15)] hover:bg-[rgba(255,215,0,0.18)] transition-all"
                    >
                      ▶ Start Timer
                    </button>
                  )}
                  {isTimerRunning && (
                    <button
                      type="button"
                      onClick={handleReveal}
                      className="px-3 py-1.5 text-xs rounded-lg bg-white/[0.05] text-white/50 border border-white/[0.08] hover:bg-white/[0.08] transition-all"
                    >
                      I'm Done
                    </button>
                  )}
                </div>
              </div>

              <p className="text-[11px] text-white/15 text-center">
                Think about which of your STAR stories best fits this question, then click "I'm Done"
              </p>
            </>
          )}

          {phase === "reveal" && (
            <>
              <p className="text-xs text-white/30 text-center mb-2">
                Which story would you use? Select one:
              </p>
              <div className="grid grid-cols-2 gap-2 max-h-48 overflow-y-auto">
                {stories.map((story) => (
                  <button
                    key={story.id}
                    type="button"
                    onClick={() => handleSelectStory(story.id)}
                    className={`text-left rounded-lg px-3 py-2.5 border transition-all ${
                      selectedStoryId === story.id
                        ? "bg-[rgba(255,215,0,0.06)] border-[rgba(255,215,0,0.15)]"
                        : "bg-white/[0.02] border-white/[0.04] hover:bg-white/[0.04]"
                    } ${practicedIds.has(story.id) ? "opacity-40" : ""}`}
                  >
                    <span className="text-xs font-medium text-white/50 block truncate">
                      {story.title || "Untitled"}
                    </span>
                    <span className="text-[10px] text-white/20 line-clamp-1 mt-0.5">
                      {story.situation.slice(0, 60)}...
                    </span>
                    {parseTags(story.tags).length > 0 && (
                      <div className="flex gap-1 mt-1">
                        {parseTags(story.tags).slice(0, 2).map((t) => (
                          <span key={t} className="text-[9px] text-[#ffd700]/25">#{t}</span>
                        ))}
                      </div>
                    )}
                  </button>
                ))}
              </div>
              <div className="flex justify-center gap-2">
                <button
                  type="button"
                  onClick={handleMarkPracticed}
                  disabled={!selectedStoryId}
                  className="px-4 py-2 text-xs rounded-lg bg-[rgba(255,215,0,0.1)] text-[#ffd700] border border-[rgba(255,215,0,0.15)] hover:bg-[rgba(255,215,0,0.18)] disabled:opacity-30 disabled:cursor-not-allowed transition-all"
                >
                  Mark Practiced & Continue
                </button>
                <button
                  type="button"
                  onClick={handleNext}
                  className="px-4 py-2 text-xs rounded-lg bg-white/[0.03] text-white/40 border border-white/[0.06] hover:bg-white/[0.05] transition-all"
                >
                  Skip →
                </button>
              </div>
            </>
          )}

          {phase === "recap" && selectedStory && (
            <>
              <div className="bg-[rgba(255,215,0,0.03)] border border-[rgba(255,215,0,0.08)] rounded-xl px-5 py-4">
                <p className="text-[10px] text-[#ffd700]/30 uppercase tracking-wider mb-2">
                  Your Answer — {selectedStory.title || "Untitled"}
                </p>
                <div className="space-y-2 text-xs">
                  <p><span className="text-blue-400/50 font-medium">Situation:</span> <span className="text-white/50">{selectedStory.situation}</span></p>
                  <p><span className="text-amber-400/50 font-medium">Task:</span> <span className="text-white/50">{selectedStory.task}</span></p>
                  <p><span className="text-emerald-400/50 font-medium">Action:</span> <span className="text-white/50">{selectedStory.action}</span></p>
                  <p><span className="text-rose-400/50 font-medium">Result:</span> <span className="text-white/50">{selectedStory.result}</span></p>
                </div>
              </div>
              <div className="text-center">
                <p className="text-xs text-white/20 mb-3">
                  Time: {formatTime(timerSeconds)} — Question: "{currentQuestion}"
                </p>
                <button
                  type="button"
                  onClick={handleNext}
                  className="px-5 py-2 text-xs rounded-lg bg-[rgba(255,215,0,0.1)] text-[#ffd700] border border-[rgba(255,215,0,0.15)] hover:bg-[rgba(255,215,0,0.18)] transition-all"
                >
                  Next Question →
                </button>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
