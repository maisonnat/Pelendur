import type { StarStoryRecord } from "../../lib/ipc";

interface StoryListProps {
  stories: StarStoryRecord[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onNew: () => void;
  searchQuery: string;
  onSearchChange: (q: string) => void;
}

function parseTags(tags: string | null): string[] {
  if (!tags) return [];
  try {
    return JSON.parse(tags);
  } catch {
    return [];
  }
}

export default function StoryList({ stories, selectedId, onSelect, onNew, searchQuery, onSearchChange }: StoryListProps) {
  const filtered = stories.filter((s) => {
    if (!searchQuery.trim()) return true;
    const q = searchQuery.toLowerCase();
    const tags = parseTags(s.tags);
    return (
      (s.title?.toLowerCase().includes(q) ?? false) ||
      s.situation.toLowerCase().includes(q) ||
      s.action.toLowerCase().includes(q) ||
      tags.some((t) => t.toLowerCase().includes(q))
    );
  });

  const STAKES_COLORS: Record<string, string> = {
    high: "bg-rose-500/15 text-rose-400/60",
    medium: "bg-amber-500/15 text-amber-400/60",
    low: "bg-emerald-500/15 text-emerald-400/60",
  };

  return (
    <div className="flex flex-col h-full">
      <div className="px-4 pt-4 pb-3 space-y-3">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-medium text-white/50 uppercase tracking-wider">
            Stories ({filtered.length})
          </h2>
          <button
            type="button"
            onClick={onNew}
            className="px-2.5 py-1 text-[11px] rounded-md bg-[rgba(255,215,0,0.1)] text-[#ffd700] border border-[rgba(255,215,0,0.15)] hover:bg-[rgba(255,215,0,0.18)] transition-all"
          >
            + New
          </button>
        </div>

        <div className="relative">
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => onSearchChange(e.target.value)}
            placeholder="Search stories..."
            className="w-full bg-white/[0.03] border border-white/[0.06] rounded-md px-3 py-1.5 text-xs text-white/60 placeholder-white/15 focus:outline-none focus:border-[#ffd700]/25 transition-colors"
          />
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-2 space-y-1 pb-4">
        {filtered.length === 0 && (
          <div className="px-3 py-8 text-center">
            <p className="text-xs text-white/15">
              {searchQuery ? "No matching stories" : "No stories yet. Create one!"}
            </p>
          </div>
        )}
        {filtered.map((story) => {
          const tags = parseTags(story.tags);
          const isSelected = story.id === selectedId;
          return (
            <button
              key={story.id}
              type="button"
              onClick={() => onSelect(story.id)}
              className={`w-full text-left rounded-lg px-3 py-2.5 transition-all duration-150 ${
                isSelected
                  ? "bg-[rgba(255,215,0,0.06)] border border-[rgba(255,215,0,0.12)]"
                  : "hover:bg-white/[0.02] border border-transparent"
              }`}
            >
              <div className="flex items-start justify-between gap-2">
                <span className="text-xs font-medium text-white/60 truncate">
                  {story.title || "Untitled"}
                </span>
                {story.stakes && (
                  <span className={`shrink-0 px-1.5 py-0.5 rounded text-[9px] font-medium ${STAKES_COLORS[story.stakes] ?? "bg-white/5 text-white/20"}`}>
                    {story.stakes}
                  </span>
                )}
              </div>
              <p className="text-[11px] text-white/20 mt-1 line-clamp-2 leading-relaxed">
                {story.situation}
              </p>
              {tags.length > 0 && (
                <div className="flex flex-wrap gap-1 mt-1.5">
                  {tags.slice(0, 3).map((tag) => (
                    <span
                      key={tag}
                      className="text-[9px] text-[#ffd700]/30"
                    >
                      #{tag}
                    </span>
                  ))}
                  {tags.length > 3 && (
                    <span className="text-[9px] text-white/10">+{tags.length - 3}</span>
                  )}
                </div>
              )}
            </button>
          );
        })}
      </div>
    </div>
  );
}
