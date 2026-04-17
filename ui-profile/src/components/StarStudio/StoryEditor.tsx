import { type StoryFormData, EMPTY_STORY, COMPETENCY_TEMPLATES } from "./types";
import TagInput from "./TagInput";

interface StoryEditorProps {
  story: StoryFormData;
  onChange: (story: StoryFormData) => void;
  onSave: () => void;
  onCancel: () => void;
  isNew: boolean;
}

const FIELDS: { key: keyof Pick<StoryFormData, "situation" | "task" | "action" | "result">; label: string; letter: string; color: string }[] = [
  { key: "situation", label: "Situation", letter: "S", color: "bg-blue-500/20 text-blue-400 border-blue-500/20" },
  { key: "task", label: "Task", letter: "T", color: "bg-amber-500/20 text-amber-400 border-amber-500/20" },
  { key: "action", label: "Action", letter: "A", color: "bg-emerald-500/20 text-emerald-400 border-emerald-500/20" },
  { key: "result", label: "Result", letter: "R", color: "bg-rose-500/20 text-rose-400 border-rose-500/20" },
];

export default function StoryEditor({ story, onChange, onSave, onCancel, isNew }: StoryEditorProps) {
  const handleFieldChange = (key: keyof StoryFormData, value: string | string[]) => {
    onChange({ ...story, [key]: value });
  };

  const applyTemplate = (key: string) => {
    const tmpl = COMPETENCY_TEMPLATES[key];
    if (tmpl) {
      onChange({
        ...EMPTY_STORY,
        ...tmpl,
        title: story.title,
        tags: tmpl.tags ?? [],
      });
    }
  };

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium text-white/40 uppercase tracking-wider">
          {isNew ? "New Story" : "Edit Story"}
        </h3>
        {isNew && (
          <div className="flex items-center gap-1.5">
            <span className="text-[10px] text-white/20 uppercase tracking-wider">Templates:</span>
            {Object.keys(COMPETENCY_TEMPLATES).map((key) => (
              <button
                key={key}
                type="button"
                onClick={() => applyTemplate(key)}
                className="px-2 py-0.5 text-[10px] rounded-md bg-white/[0.03] text-white/30 hover:text-[#ffd700]/70 hover:bg-[rgba(255,215,0,0.05)] border border-white/[0.04] hover:border-[rgba(255,215,0,0.15)] transition-all"
              >
                {key}
              </button>
            ))}
          </div>
        )}
      </div>

      <div>
        <input
          type="text"
          value={story.title}
          onChange={(e) => handleFieldChange("title", e.target.value)}
          placeholder="Story title (e.g. 'Led migration to microservices')"
          className="w-full bg-white/[0.03] border border-white/[0.08] rounded-lg px-3.5 py-2.5 text-sm text-white/80 placeholder-white/15 focus:outline-none focus:border-[#ffd700]/30 focus:bg-white/[0.05] transition-colors"
        />
      </div>

      <div className="space-y-3">
        {FIELDS.map(({ key, label, letter, color }) => (
          <div key={key}>
            <div className="flex items-center gap-2 mb-1.5">
              <span className={`inline-flex items-center justify-center w-5 h-5 rounded text-[10px] font-bold border ${color}`}>
                {letter}
              </span>
              <label className="text-xs text-white/40">{label}</label>
            </div>
            <textarea
              value={story[key]}
              onChange={(e) => handleFieldChange(key, e.target.value)}
              placeholder={`Describe the ${label.toLowerCase()}...`}
              rows={3}
              className="w-full bg-white/[0.03] border border-white/[0.08] rounded-lg px-3.5 py-2.5 text-sm text-white/80 placeholder-white/15 focus:outline-none focus:border-[#ffd700]/30 focus:bg-white/[0.05] transition-colors resize-none"
            />
          </div>
        ))}
      </div>

      <div className="grid grid-cols-2 gap-3">
        <div>
          <label className="text-xs text-white/40 mb-1.5 block">Difficulty</label>
          <select
            value={story.difficulty}
            onChange={(e) => handleFieldChange("difficulty", e.target.value)}
            className="w-full bg-white/[0.03] border border-white/[0.08] rounded-lg px-3 py-2 text-xs text-white/70 focus:outline-none focus:border-[#ffd700]/30 transition-colors appearance-none"
          >
            <option value="">Not set</option>
            <option value="easy">Easy</option>
            <option value="medium">Medium</option>
            <option value="hard">Hard</option>
          </select>
        </div>
        <div>
          <label className="text-xs text-white/40 mb-1.5 block">Stakes</label>
          <select
            value={story.stakes}
            onChange={(e) => handleFieldChange("stakes", e.target.value)}
            className="w-full bg-white/[0.03] border border-white/[0.08] rounded-lg px-3 py-2 text-xs text-white/70 focus:outline-none focus:border-[#ffd700]/30 transition-colors appearance-none"
          >
            <option value="">Not set</option>
            <option value="low">Low</option>
            <option value="medium">Medium</option>
            <option value="high">High</option>
          </select>
        </div>
      </div>

      <div>
        <label className="text-xs text-white/40 mb-1.5 block">Tags</label>
        <TagInput tags={story.tags} onChange={(tags) => handleFieldChange("tags", tags)} />
      </div>

      <div className="flex items-center gap-2 pt-2">
        <button
          type="button"
          onClick={onSave}
          disabled={!story.situation.trim() || !story.task.trim() || !story.action.trim() || !story.result.trim()}
          className="px-4 py-2 text-xs font-medium rounded-lg bg-[rgba(255,215,0,0.12)] text-[#ffd700] border border-[rgba(255,215,0,0.2)] hover:bg-[rgba(255,215,0,0.18)] disabled:opacity-30 disabled:cursor-not-allowed transition-all"
        >
          {isNew ? "Create Story" : "Save Changes"}
        </button>
        <button
          type="button"
          onClick={onCancel}
          className="px-4 py-2 text-xs font-medium rounded-lg bg-white/[0.03] text-white/40 border border-white/[0.06] hover:text-white/60 hover:bg-white/[0.05] transition-all"
        >
          Cancel
        </button>
      </div>
    </div>
  );
}
