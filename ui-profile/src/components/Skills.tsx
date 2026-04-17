import { useState, useEffect, useCallback } from "react";
import {
  listSkills,
  createSkill,
  updateSkill,
  deleteSkill,
  listEdgesForEntity,
  type Skill,
} from "../lib/ipc";

const LEVEL_CONFIG: Record<string, { color: string; bg: string; pct: number; label: string }> = {
  expert: { color: "#ffd700", bg: "rgba(255,215,0,0.15)", pct: 100, label: "Expert" },
  advanced: { color: "#22c55e", bg: "rgba(34,197,94,0.15)", pct: 75, label: "Advanced" },
  intermediate: { color: "#3b82f6", bg: "rgba(59,130,246,0.15)", pct: 50, label: "Intermediate" },
  learning: { color: "#6b7280", bg: "rgba(107,114,128,0.15)", pct: 25, label: "Learning" },
};

const CATEGORIES = ["Languages", "Frameworks", "Tools", "Practices", "Soft Skills"];
const LEVELS = ["expert", "advanced", "intermediate", "learning"];

interface SkillFormData {
  name: string;
  category: string;
  level: string;
  years: number;
}

const EMPTY_FORM: SkillFormData = { name: "", category: "Languages", level: "intermediate", years: 1 };

function SkillCard({
  skill,
  hasStories,
  onEdit,
  onDelete,
}: {
  skill: Skill;
  hasStories: boolean;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const cfg = LEVEL_CONFIG[skill.level] ?? LEVEL_CONFIG.learning;
  const displayCategory = skill.category || "Uncategorized";

  return (
    <div className="group bg-[rgba(30,30,30,0.85)] backdrop-blur-md rounded-lg border border-white/5 hover:border-white/10 transition-colors p-4">
      <div className="flex items-start justify-between mb-2">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <h3 className="text-sm font-medium text-white/90 truncate">{skill.name}</h3>
            {!hasStories && (
              <span className="text-amber-500/60 text-xs" title="No linked STAR stories">⚠</span>
            )}
          </div>
          <span className="text-[11px] text-white/25 uppercase tracking-wider">{displayCategory}</span>
        </div>
        <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
          <button
            onClick={onEdit}
            className="px-2 py-1 text-[11px] text-white/40 hover:text-white/70 hover:bg-white/5 rounded transition-colors"
          >
            Edit
          </button>
          <button
            onClick={onDelete}
            className="px-2 py-1 text-[11px] text-red-400/50 hover:text-red-400 hover:bg-red-400/5 rounded transition-colors"
          >
            Delete
          </button>
        </div>
      </div>

      <div className="flex items-center gap-3 mt-3">
        <div className="flex-1 h-1.5 bg-white/5 rounded-full overflow-hidden">
          <div
            className="h-full rounded-full transition-all duration-500"
            style={{ width: `${cfg.pct}%`, backgroundColor: cfg.color }}
          />
        </div>
        <span
          className="text-[11px] font-medium px-1.5 py-0.5 rounded"
          style={{ color: cfg.color, backgroundColor: cfg.bg }}
        >
          {cfg.label}
        </span>
      </div>

      <div className="mt-2 text-[11px] text-white/20">
        {skill.years} {skill.years === 1 ? "year" : "years"}
      </div>
    </div>
  );
}

function SkillModal({
  initial,
  onSave,
  onClose,
}: {
  initial: SkillFormData & { id?: string };
  onSave: (data: SkillFormData & { id?: string }) => void;
  onClose: () => void;
}) {
  const [form, setForm] = useState<SkillFormData>(initial);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" onClick={onClose}>
      <div
        className="w-full max-w-md bg-[rgba(20,20,20,0.95)] backdrop-blur-md rounded-xl border border-white/10 p-6"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="text-sm font-semibold text-[#ffd700] mb-4">
          {initial.id ? "Edit Skill" : "Add Skill"}
        </h3>

        <div className="space-y-3">
          <div>
            <label className="text-[11px] text-white/30 uppercase tracking-wider">Name</label>
            <input
              type="text"
              value={form.name}
              onChange={(e) => setForm({ ...form, name: e.target.value })}
              className="w-full mt-1 bg-white/[0.04] border border-white/[0.06] rounded-md px-3 py-2 text-sm text-white/80 focus:outline-none focus:border-[#ffd700]/30"
              autoFocus
            />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="text-[11px] text-white/30 uppercase tracking-wider">Category</label>
              <select
                value={form.category}
                onChange={(e) => setForm({ ...form, category: e.target.value })}
                className="w-full mt-1 bg-white/[0.04] border border-white/[0.06] rounded-md px-3 py-2 text-sm text-white/80 focus:outline-none focus:border-[#ffd700]/30"
              >
                {CATEGORIES.map((c) => (
                  <option key={c} value={c} className="bg-gray-900">{c}</option>
                ))}
              </select>
            </div>
            <div>
              <label className="text-[11px] text-white/30 uppercase tracking-wider">Level</label>
              <select
                value={form.level}
                onChange={(e) => setForm({ ...form, level: e.target.value })}
                className="w-full mt-1 bg-white/[0.04] border border-white/[0.06] rounded-md px-3 py-2 text-sm text-white/80 focus:outline-none focus:border-[#ffd700]/30"
              >
                {LEVELS.map((l) => (
                  <option key={l} value={l} className="bg-gray-900">
                    {LEVEL_CONFIG[l]?.label ?? l}
                  </option>
                ))}
              </select>
            </div>
          </div>

          <div>
            <label className="text-[11px] text-white/30 uppercase tracking-wider">Years of Experience</label>
            <input
              type="number"
              min={0}
              max={50}
              value={form.years}
              onChange={(e) => setForm({ ...form, years: Math.max(0, parseInt(e.target.value) || 0) })}
              className="w-full mt-1 bg-white/[0.04] border border-white/[0.06] rounded-md px-3 py-2 text-sm text-white/80 focus:outline-none focus:border-[#ffd700]/30"
            />
          </div>
        </div>

        <div className="flex justify-end gap-2 mt-5">
          <button
            onClick={onClose}
            className="px-4 py-2 text-xs text-white/40 hover:text-white/60 rounded-lg transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={() => form.name.trim() && onSave({ ...form, id: initial.id })}
            disabled={!form.name.trim()}
            className="px-4 py-2 text-xs bg-[#ffd700]/10 text-[#ffd700] rounded-lg hover:bg-[#ffd700]/20 transition-colors disabled:opacity-30"
          >
            {initial.id ? "Update" : "Add"}
          </button>
        </div>
      </div>
    </div>
  );
}

export default function Skills() {
  const [skills, setSkills] = useState<Skill[]>([]);
  const [loading, setLoading] = useState(true);
  const [modal, setModal] = useState<SkillFormData & { id?: string } | null>(null);
  const [skillEdgeMap, setSkillEdgeMap] = useState<Record<string, boolean>>({});

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const list = await listSkills();
      setSkills(list);

      const edgeMap: Record<string, boolean> = {};
      await Promise.all(
        list.map(async (s) => {
          const edges = await listEdgesForEntity(s.id, "skill");
          const hasStar = edges.some(
            (e) => e.source_type === "star_story" || e.target_type === "star_story"
          );
          edgeMap[s.id] = hasStar;
        })
      );
      setSkillEdgeMap(edgeMap);
    } catch {
      setSkills([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const handleSave = async (data: SkillFormData & { id?: string }) => {
    if (data.id) {
      await updateSkill({ id: data.id, name: data.name, category: data.category, level: data.level, years: data.years });
    } else {
      await createSkill({ name: data.name, category: data.category, level: data.level, years: data.years });
    }
    setModal(null);
    load();
  };

  const handleDelete = async (id: string) => {
    await deleteSkill(id);
    load();
  };

  const grouped = CATEGORIES.reduce<Record<string, Skill[]>>((acc, cat) => {
    acc[cat] = skills.filter((s) => (s.category || "Uncategorized") === cat);
    return acc;
  }, {});
  const uncategorized = skills.filter((s) => s.category && !CATEGORIES.includes(s.category));
  if (uncategorized.length > 0) {
    grouped["Other"] = uncategorized;
  }

  return (
    <div className="max-w-4xl space-y-5">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-[#ffd700]">Skills</h2>
        <button
          onClick={() => setModal({ ...EMPTY_FORM })}
          className="px-3 py-1.5 text-xs bg-[#ffd700]/10 text-[#ffd700] rounded-lg hover:bg-[#ffd700]/20 transition-colors"
        >
          + Add Skill
        </button>
      </div>

      {loading ? (
        <div className="text-white/20 text-sm py-12 text-center">Loading skills…</div>
      ) : skills.length === 0 ? (
        <div className="bg-[rgba(30,30,30,0.85)] backdrop-blur-md rounded-xl border border-white/5 p-8 text-center">
          <p className="text-white/30 text-sm">No skills yet. Add your first skill to get started.</p>
        </div>
      ) : (
        Object.entries(grouped).map(
          ([cat, items]) =>
            items.length > 0 && (
              <div key={cat}>
                <h3 className="text-[11px] text-white/25 uppercase tracking-widest mb-2">{cat}</h3>
                <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
                  {items.map((skill) => (
                    <SkillCard
                      key={skill.id}
                      skill={skill}
                      hasStories={!!skillEdgeMap[skill.id]}
                      onEdit={() =>
                        setModal({
                          id: skill.id,
                          name: skill.name,
                          category: skill.category || "Languages",
                          level: skill.level,
                          years: skill.years,
                        })
                      }
                      onDelete={() => handleDelete(skill.id)}
                    />
                  ))}
                </div>
              </div>
            )
        )
      )}

      {modal && <SkillModal initial={modal} onSave={handleSave} onClose={() => setModal(null)} />}
    </div>
  );
}
