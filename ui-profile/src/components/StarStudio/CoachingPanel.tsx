import { useState, useRef, useEffect } from "react";
import { coachStarStory } from "../../lib/ipc";

interface CoachingPanelProps {
  storyId: string | null;
  storyTitle: string;
  onClose: () => void;
}

interface Message {
  role: "user" | "coach";
  content: string;
}

const COACHING_PRESETS = [
  "How can I make this story more impactful?",
  "Is my Result section specific enough?",
  "What questions might an interviewer ask about this?",
  "Help me add quantifiable metrics",
  "How can I tighten the narrative?",
];

export default function CoachingPanel({ storyId, storyTitle, onClose }: CoachingPanelProps) {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: "smooth" });
  }, [messages]);

  const askCoach = async (question: string) => {
    if (!question.trim() || loading) return;
    const userMsg: Message = { role: "user", content: question };
    setMessages((prev) => [...prev, userMsg]);
    setInput("");
    setLoading(true);

    try {
      const response = await coachStarStory(storyId, question);
      setMessages((prev) => [...prev, { role: "coach", content: response }]);
    } catch (e) {
      setMessages((prev) => [
        ...prev,
        { role: "coach", content: "Sorry, I couldn't process your request. Please try again." },
      ]);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex flex-col h-full">
      <div className="px-4 pt-4 pb-3 flex items-center justify-between border-b border-white/[0.04]">
        <div>
          <h3 className="text-sm font-medium text-[#ffd700]/70">AI Coach</h3>
          <p className="text-[10px] text-white/20 mt-0.5 truncate max-w-[200px]">
            {storyTitle || "General coaching"}
          </p>
        </div>
        <button
          type="button"
          onClick={onClose}
          className="text-white/20 hover:text-white/50 transition-colors text-xs"
        >
          ✕
        </button>
      </div>

      <div ref={scrollRef} className="flex-1 overflow-y-auto px-4 py-3 space-y-3">
        {messages.length === 0 && (
          <div className="space-y-2">
            <p className="text-[11px] text-white/20 text-center py-2">
              Ask for coaching suggestions
            </p>
            <div className="space-y-1">
              {COACHING_PRESETS.map((preset) => (
                <button
                  key={preset}
                  type="button"
                  onClick={() => askCoach(preset)}
                  className="w-full text-left px-3 py-2 text-[11px] text-white/30 rounded-lg bg-white/[0.02] border border-white/[0.04] hover:text-white/50 hover:bg-white/[0.04] hover:border-white/[0.08] transition-all"
                >
                  {preset}
                </button>
              ))}
            </div>
          </div>
        )}

        {messages.map((msg, i) => (
          <div
            key={i}
            className={`rounded-lg px-3 py-2.5 text-xs leading-relaxed ${
              msg.role === "user"
                ? "bg-white/[0.03] text-white/50 ml-6"
                : "bg-[rgba(255,215,0,0.04)] text-white/60 mr-2 border border-[rgba(255,215,0,0.06)]"
            }`}
          >
            {msg.role === "coach" && (
              <span className="text-[9px] text-[#ffd700]/40 font-medium uppercase tracking-wider">
                Coach
              </span>
            )}
            <p className="mt-0.5 whitespace-pre-wrap">{msg.content}</p>
          </div>
        ))}

        {loading && (
          <div className="bg-[rgba(255,215,0,0.04)] border border-[rgba(255,215,0,0.06)] rounded-lg px-3 py-2.5 mr-2">
            <span className="text-[9px] text-[#ffd700]/40 font-medium uppercase tracking-wider">
              Coach
            </span>
            <div className="flex items-center gap-1.5 mt-1.5">
              <span className="w-1.5 h-1.5 rounded-full bg-[#ffd700]/30 animate-bounce" style={{ animationDelay: "0ms" }} />
              <span className="w-1.5 h-1.5 rounded-full bg-[#ffd700]/30 animate-bounce" style={{ animationDelay: "150ms" }} />
              <span className="w-1.5 h-1.5 rounded-full bg-[#ffd700]/30 animate-bounce" style={{ animationDelay: "300ms" }} />
            </div>
          </div>
        )}
      </div>

      <div className="px-3 py-3 border-t border-white/[0.04]">
        <form
          onSubmit={(e) => {
            e.preventDefault();
            askCoach(input);
          }}
          className="flex gap-2"
        >
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder="Ask the coach..."
            disabled={loading}
            className="flex-1 bg-white/[0.03] border border-white/[0.06] rounded-md px-3 py-1.5 text-xs text-white/60 placeholder-white/15 focus:outline-none focus:border-[#ffd700]/25 disabled:opacity-30 transition-colors"
          />
          <button
            type="submit"
            disabled={loading || !input.trim()}
            className="px-3 py-1.5 text-xs rounded-md bg-[rgba(255,215,0,0.08)] text-[#ffd700]/60 border border-[rgba(255,215,0,0.1)] hover:bg-[rgba(255,215,0,0.14)] disabled:opacity-20 disabled:cursor-not-allowed transition-all"
          >
            Send
          </button>
        </form>
      </div>
    </div>
  );
}
