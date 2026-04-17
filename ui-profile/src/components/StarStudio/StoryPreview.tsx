import type { StarStoryRecord } from "../../lib/ipc";

interface StoryPreviewProps {
  story: StarStoryRecord;
  onEdit: () => void;
  onDelete: () => void;
  onClose: () => void;
  onCoach: () => void;
}

function parseTags(tags: string | null): string[] {
  if (!tags) return [];
  try {
    return JSON.parse(tags);
  } catch {
    return [];
  }
}

const STAR_SECTIONS = [
  { key: "situation" as const, label: "Situation", letter: "S", border: "border-l-blue-500/40", bg: "bg-blue-500/[0.03]" },
  { key: "task" as const, label: "Task", letter: "T", border: "border-l-amber-500/40", bg: "bg-amber-500/[0.03]" },
  { key: "action" as const, label: "Action", letter: "A", border: "border-l-emerald-500/40", bg: "bg-emerald-500/[0.03]" },
  { key: "result" as const, label: "Result", letter: "R", border: "border-l-rose-500/40", bg: "bg-rose-500/[0.03]" },
];

export default function StoryPreview({ story, onEdit, onDelete, onClose, onCoach }: StoryPreviewProps) {
  const tags = parseTags(story.tags);

  return (
    <div className="space-y-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h3 className="text-base font-medium text-white/80">
            {story.title || "Untitled Story"}
          </h3>
          <div className="flex items-center gap-3 mt-1.5">
            {story.difficulty && (
              <span className="text-[10px] uppercase tracking-wider text-white/25">
                {story.difficulty}
              </span>
            )}
            {story.stakes && (
              <span className="text-[10px] uppercase tracking-wider text-white/25">
                stakes: {story.stakes}
              </span>
            )}
            <span className="text-[10px] text-white/15">
              Used {story.usage_count}×
            </span>
          </div>
        </div>
        <button
          type="button"
          onClick={onClose}
          className="text-white/20 hover:text-white/50 transition-colors text-sm"
        >
          ✕
        </button>
      </div>

      {tags.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {tags.map((tag) => (
            <span
              key={tag}
              className="px-2 py-0.5 rounded-md text-[10px] font-medium bg-[rgba(255,215,0,0.06)] text-[#ffd700]/60 border border-[rgba(255,215,0,0.1)]"
            >
              {tag}
            </span>
          ))}
        </div>
      )}

      <div className="space-y-2.5">
        {STAR_SECTIONS.map(({ key, label, letter, border, bg }) => (
          <div
            key={key}
            className={`border-l-2 ${border} ${bg} rounded-r-lg px-4 py-3`}
          >
            <div className="flex items-center gap-2 mb-1">
              <span className="text-[10px] font-bold text-white/30">{letter}</span>
              <span className="text-[11px] uppercase tracking-wider text-white/30">{label}</span>
            </div>
            <p className="text-sm text-white/65 leading-relaxed">{story[key]}</p>
          </div>
        ))}
      </div>

      <div className="flex items-center gap-2 pt-2 border-t border-white/[0.04]">
        <button
          type="button"
          onClick={onCoach}
          className="px-3 py-1.5 text-xs rounded-lg bg-[rgba(255,215,0,0.08)] text-[#ffd700]/70 border border-[rgba(255,215,0,0.12)] hover:bg-[rgba(255,215,0,0.14)] transition-all"
        >
          ✦ AI Coach
        </button>
        <button
          type="button"
          onClick={onEdit}
          className="px-3 py-1.5 text-xs rounded-lg bg-white/[0.03] text-white/40 border border-white/[0.06] hover:text-white/60 hover:bg-white/[0.05] transition-all"
        >
          Edit
        </button>
        <button
          type="button"
          onClick={onDelete}
          className="px-3 py-1.5 text-xs rounded-lg text-red-400/40 hover:text-red-400/70 hover:bg-red-500/[0.05] transition-all"
        >
          Delete
        </button>
      </div>
    </div>
  );
}
