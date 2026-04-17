import { useState, useEffect, useCallback } from "react";
import CareerTimeline from "./Timeline/CareerTimeline";
import {
  listExperiencesWithSkills,
  createExperience,
  updateExperience,
  deleteExperience,
  listSkills,
  type ExperienceWithSkills,
  type Skill,
} from "../lib/ipc";

type ViewMode = "timeline" | "cards";

interface ExpFormData {
  id?: string;
  company: string;
  role: string;
  start_date: string;
  end_date: string;
  description: string;
  highlights: string;
  selectedSkillIds: string[];
}

const EMPTY_FORM: ExpFormData = {
  company: "",
  role: "",
  start_date: "",
  end_date: "",
  description: "",
  highlights: "",
  selectedSkillIds: [],
};

function formatDate(date: string | null): string {
  if (!date) return "Present";
  if (date.length === 7) {
    const [y, m] = date.split("-");
    const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    return `${months[parseInt(m, 10) - 1]} ${y}`;
  }
  return date;
}

function ExperienceCard({
  exp,
  onEdit,
  onDelete,
}: {
  exp: ExperienceWithSkills;
  onEdit: () => void;
  onDelete: () => void;
}) {
  return (
    <div className="group bg-[rgba(30,30,30,0.85)] backdrop-blur-md rounded-lg border border-white/5 hover:border-white/10 transition-colors p-4">
      <div className="flex items-start justify-between mb-1">
        <div className="min-w-0 flex-1">
          <h3 className="text-sm font-medium text-white/90">{exp.role}</h3>
          <p className="text-[#ffd700]/70 text-xs">{exp.company}</p>
        </div>
        <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
          <button onClick={onEdit} className="px-2 py-1 text-[11px] text-white/40 hover:text-white/70 hover:bg-white/5 rounded transition-colors">Edit</button>
          <button onClick={onDelete} className="px-2 py-1 text-[11px] text-red-400/50 hover:text-red-400 hover:bg-red-400/5 rounded transition-colors">Delete</button>
        </div>
      </div>

      <p className="text-[11px] text-white/25 mb-2">
        {formatDate(exp.start_date)} — {formatDate(exp.end_date)}
      </p>

      {exp.description && (
        <p className="text-xs text-white/40 leading-relaxed mb-2">{exp.description}</p>
      )}

      {exp.skills.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {exp.skills.map((skill) => (
            <span key={skill} className="text-[10px] px-2 py-0.5 rounded-full bg-[rgba(255,215,0,0.1)] text-[#ffd700]/70 border border-[#ffd700]/20">
              {skill}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

function ExperienceModal({
  initial,
  allSkills,
  onSave,
  onClose,
}: {
  initial: ExpFormData;
  allSkills: Skill[];
  onSave: (data: ExpFormData) => void;
  onClose: () => void;
}) {
  const [form, setForm] = useState<ExpFormData>(initial);

  const toggleSkill = (skillId: string) => {
    setForm((prev) => ({
      ...prev,
      selectedSkillIds: prev.selectedSkillIds.includes(skillId)
        ? prev.selectedSkillIds.filter((id) => id !== skillId)
        : [...prev.selectedSkillIds, skillId],
    }));
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" onClick={onClose}>
      <div
        className="w-full max-w-lg max-h-[85vh] overflow-y-auto bg-[rgba(20,20,20,0.95)] backdrop-blur-md rounded-xl border border-white/10 p-6"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 className="text-sm font-semibold text-[#ffd700] mb-4">
          {initial.id ? "Edit Experience" : "Add Experience"}
        </h3>

        <div className="space-y-3">
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="text-[11px] text-white/30 uppercase tracking-wider">Company</label>
              <input
                type="text"
                value={form.company}
                onChange={(e) => setForm({ ...form, company: e.target.value })}
                className="w-full mt-1 bg-white/[0.04] border border-white/[0.06] rounded-md px-3 py-2 text-sm text-white/80 focus:outline-none focus:border-[#ffd700]/30"
                autoFocus
              />
            </div>
            <div>
              <label className="text-[11px] text-white/30 uppercase tracking-wider">Role</label>
              <input
                type="text"
                value={form.role}
                onChange={(e) => setForm({ ...form, role: e.target.value })}
                className="w-full mt-1 bg-white/[0.04] border border-white/[0.06] rounded-md px-3 py-2 text-sm text-white/80 focus:outline-none focus:border-[#ffd700]/30"
              />
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="text-[11px] text-white/30 uppercase tracking-wider">Start Date</label>
              <input
                type="month"
                value={form.start_date}
                onChange={(e) => setForm({ ...form, start_date: e.target.value })}
                className="w-full mt-1 bg-white/[0.04] border border-white/[0.06] rounded-md px-3 py-2 text-sm text-white/80 focus:outline-none focus:border-[#ffd700]/30"
              />
            </div>
            <div>
              <label className="text-[11px] text-white/30 uppercase tracking-wider">End Date</label>
              <input
                type="month"
                value={form.end_date}
                onChange={(e) => setForm({ ...form, end_date: e.target.value })}
                className="w-full mt-1 bg-white/[0.04] border border-white/[0.06] rounded-md px-3 py-2 text-sm text-white/80 focus:outline-none focus:border-[#ffd700]/30"
                placeholder="Leave empty for current"
              />
            </div>
          </div>

          <div>
            <label className="text-[11px] text-white/30 uppercase tracking-wider">Description</label>
            <textarea
              value={form.description}
              onChange={(e) => setForm({ ...form, description: e.target.value })}
              rows={3}
              className="w-full mt-1 bg-white/[0.04] border border-white/[0.06] rounded-md px-3 py-2 text-sm text-white/80 focus:outline-none focus:border-[#ffd700]/30 resize-none"
            />
          </div>

          <div>
            <label className="text-[11px] text-white/30 uppercase tracking-wider">Highlights (comma-separated)</label>
            <input
              type="text"
              value={form.highlights}
              onChange={(e) => setForm({ ...form, highlights: e.target.value })}
              className="w-full mt-1 bg-white/[0.04] border border-white/[0.06] rounded-md px-3 py-2 text-sm text-white/80 focus:outline-none focus:border-[#ffd700]/30"
              placeholder="Led migration, Shipped v1, Mentored 3 devs"
            />
          </div>

          <div>
            <label className="text-[11px] text-white/30 uppercase tracking-wider mb-1 block">Linked Skills</label>
            <div className="flex flex-wrap gap-1.5 max-h-32 overflow-y-auto p-2 bg-white/[0.02] rounded-md border border-white/[0.04]">
              {allSkills.length === 0 && (
                <span className="text-[11px] text-white/20">No skills available. Add skills first.</span>
              )}
              {allSkills.map((skill) => {
                const selected = form.selectedSkillIds.includes(skill.id);
                return (
                  <button
                    key={skill.id}
                    type="button"
                    onClick={() => toggleSkill(skill.id)}
                    className={`text-[10px] px-2 py-0.5 rounded-full border transition-colors ${
                      selected
                        ? "bg-[rgba(255,215,0,0.15)] text-[#ffd700]/90 border-[#ffd700]/30"
                        : "bg-transparent text-white/30 border-white/10 hover:border-white/20"
                    }`}
                  >
                    {skill.name}
                  </button>
                );
              })}
            </div>
          </div>
        </div>

        <div className="flex justify-end gap-2 mt-5">
          <button onClick={onClose} className="px-4 py-2 text-xs text-white/40 hover:text-white/60 rounded-lg transition-colors">
            Cancel
          </button>
          <button
            onClick={() => form.company.trim() && form.role.trim() && onSave(form)}
            disabled={!form.company.trim() || !form.role.trim()}
            className="px-4 py-2 text-xs bg-[#ffd700]/10 text-[#ffd700] rounded-lg hover:bg-[#ffd700]/20 transition-colors disabled:opacity-30"
          >
            {initial.id ? "Update" : "Add"}
          </button>
        </div>
      </div>
    </div>
  );
}

export default function Experiences() {
  const [view, setView] = useState<ViewMode>("timeline");
  const [experiences, setExperiences] = useState<ExperienceWithSkills[]>([]);
  const [allSkills, setAllSkills] = useState<Skill[]>([]);
  const [modal, setModal] = useState<ExpFormData | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  const loadData = useCallback(async () => {
    const [expList, skillList] = await Promise.all([
      listExperiencesWithSkills(),
      listSkills(),
    ]);
    setExperiences(expList);
    setAllSkills(skillList);
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const handleSave = async (data: ExpFormData) => {
    const highlightsJson = data.highlights.trim()
      ? JSON.stringify(data.highlights.split(",").map((s) => s.trim()).filter(Boolean))
      : undefined;

    if (data.id) {
      await updateExperience({
        id: data.id,
        company: data.company,
        role: data.role,
        start_date: data.start_date,
        end_date: data.end_date || undefined,
        description: data.description || undefined,
        highlights: highlightsJson,
        skill_ids: data.selectedSkillIds,
      });
    } else {
      await createExperience({
        company: data.company,
        role: data.role,
        start_date: data.start_date,
        end_date: data.end_date || undefined,
        description: data.description || undefined,
        highlights: highlightsJson,
        skill_ids: data.selectedSkillIds,
      });
    }
    setModal(null);
    setRefreshKey((k) => k + 1);
    loadData();
  };

  const handleDelete = async (id: string) => {
    await deleteExperience(id);
    setRefreshKey((k) => k + 1);
    loadData();
  };

  const openEdit = (exp: ExperienceWithSkills) => {
    const matchedSkillIds = allSkills
      .filter((s) => exp.skills.includes(s.name))
      .map((s) => s.id);

    setModal({
      id: exp.id,
      company: exp.company,
      role: exp.role,
      start_date: exp.start_date,
      end_date: exp.end_date || "",
      description: exp.description || "",
      highlights: "",
      selectedSkillIds: matchedSkillIds,
    });
  };

  return (
    <div className="max-w-4xl space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-[#ffd700]">Experiences</h2>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setModal({ ...EMPTY_FORM })}
            className="px-3 py-1.5 text-xs bg-[#ffd700]/10 text-[#ffd700] rounded-lg hover:bg-[#ffd700]/20 transition-colors"
          >
            + Add Experience
          </button>
          <div className="flex bg-[rgba(30,30,30,0.85)] rounded-lg border border-white/5 p-0.5">
            <button
              onClick={() => setView("timeline")}
              className={`px-3 py-1.5 text-xs rounded-md transition-colors ${
                view === "timeline"
                  ? "bg-[rgba(255,215,0,0.1)] text-[#ffd700]"
                  : "text-white/30 hover:text-white/50"
              }`}
            >
              ◇ Timeline
            </button>
            <button
              onClick={() => setView("cards")}
              className={`px-3 py-1.5 text-xs rounded-md transition-colors ${
                view === "cards"
                  ? "bg-[rgba(255,215,0,0.1)] text-[#ffd700]"
                  : "text-white/30 hover:text-white/50"
              }`}
            >
              ⊞ Cards
            </button>
          </div>
        </div>
      </div>

      {view === "timeline" ? (
        <CareerTimeline key={refreshKey} />
      ) : experiences.length === 0 ? (
        <div className="bg-[rgba(30,30,30,0.85)] backdrop-blur-md rounded-xl border border-white/5 p-8 text-center">
          <p className="text-white/30 text-sm">No experiences yet. Add your first experience to get started.</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          {experiences.map((exp) => (
            <ExperienceCard
              key={exp.id}
              exp={exp}
              onEdit={() => openEdit(exp)}
              onDelete={() => handleDelete(exp.id)}
            />
          ))}
        </div>
      )}

      {modal && (
        <ExperienceModal
          initial={modal}
          allSkills={allSkills}
          onSave={handleSave}
          onClose={() => setModal(null)}
        />
      )}
    </div>
  );
}
