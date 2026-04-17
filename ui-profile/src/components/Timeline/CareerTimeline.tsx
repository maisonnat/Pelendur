import { useState, useEffect, useCallback, useMemo } from "react";
import { listExperiencesWithSkills, type ExperienceWithSkills } from "../../lib/ipc";

function formatDate(date: string | null): string {
  if (!date) return "Present";
  if (date.length === 4) return date;
  if (date.length === 7) {
    const [y, m] = date.split("-");
    const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    return `${months[parseInt(m, 10) - 1]} ${y}`;
  }
  return date;
}

function parseHighlights(raw: string | null): string[] {
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return raw.split(",").map((s) => s.trim()).filter(Boolean);
  }
}

function TimelineEntry({
  experience,
  side,
  onToggle,
  isExpanded,
  animationDelay,
}: {
  experience: ExperienceWithSkills;
  side: "left" | "right";
  onToggle: () => void;
  isExpanded: boolean;
  animationDelay: number;
}) {
  const highlights = useMemo(() => parseHighlights(experience.highlights), [experience.highlights]);

  return (
    <div
      className="relative flex w-full items-start opacity-0"
      style={{
        animation: `fadeInUp 0.5s ease-out ${animationDelay}ms forwards`,
      }}
    >
      <div className={`hidden lg:flex w-full items-start ${side === "right" ? "flex-row" : "flex-row-reverse"}`}>
        <div className="w-[calc(50%-2rem)]">
          <button
            onClick={onToggle}
            className="w-full text-left bg-[rgba(30,30,30,0.85)] backdrop-blur-md rounded-lg border border-white/5 p-4 hover:border-[#ffd700]/20 transition-colors cursor-pointer"
          >
            <p className="text-gray-400 text-xs mb-1">
              {formatDate(experience.start_date)} — {formatDate(experience.end_date)}
            </p>
            <h3 className="text-white/90 font-medium text-sm">{experience.role}</h3>
            <p className="text-[#ffd700]/80 text-xs mt-0.5">{experience.company}</p>

            {experience.skills.length > 0 && (
              <div className="flex flex-wrap gap-1.5 mt-2.5">
                {experience.skills.map((skill) => (
                  <span
                    key={skill}
                    className="text-[10px] px-2 py-0.5 rounded-full bg-[rgba(255,215,0,0.1)] text-[#ffd700]/70 border border-[#ffd700]/20"
                  >
                    {skill}
                  </span>
                ))}
              </div>
            )}

            {isExpanded && (experience.description || highlights.length > 0) && (
              <div className="mt-3 pt-3 border-t border-white/5 space-y-2">
                {experience.description && (
                  <p className="text-white/50 text-xs leading-relaxed">{experience.description}</p>
                )}
                {highlights.length > 0 && (
                  <ul className="space-y-1">
                    {highlights.map((h, i) => (
                      <li key={i} className="text-white/40 text-xs flex items-start gap-1.5">
                        <span className="text-[#ffd700]/40 mt-0.5 shrink-0">▸</span>
                        {h}
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            )}
          </button>
        </div>

        <div className="flex flex-col items-center w-16 shrink-0 relative">
          <div className="w-4 h-4 rounded-full border-2 border-[#ffd700]/60 bg-[rgba(10,10,10,0.95)] z-10 shrink-0" />
          {experience.end_date === null && (
            <span className="text-[8px] text-[#ffd700]/50 uppercase tracking-widest mt-1">Current</span>
          )}
        </div>

        <div className="w-[calc(50%-2rem)]" />
      </div>

      <div className="flex lg:hidden w-full items-start gap-4">
        <div className="flex flex-col items-center shrink-0">
          <div className="w-3.5 h-3.5 rounded-full border-2 border-[#ffd700]/60 bg-[rgba(10,10,10,0.95)] z-10" />
        </div>
        <button
          onClick={onToggle}
          className="flex-1 text-left bg-[rgba(30,30,30,0.85)] backdrop-blur-md rounded-lg border border-white/5 p-3 hover:border-[#ffd700]/20 transition-colors cursor-pointer"
        >
          <p className="text-gray-400 text-[10px] mb-0.5">
            {formatDate(experience.start_date)} — {formatDate(experience.end_date)}
          </p>
          <h3 className="text-white/90 font-medium text-sm">{experience.role}</h3>
          <p className="text-[#ffd700]/80 text-xs">{experience.company}</p>

          {experience.skills.length > 0 && (
            <div className="flex flex-wrap gap-1 mt-2">
              {experience.skills.map((skill) => (
                <span
                  key={skill}
                  className="text-[10px] px-1.5 py-0.5 rounded-full bg-[rgba(255,215,0,0.1)] text-[#ffd700]/70 border border-[#ffd700]/20"
                >
                  {skill}
                </span>
              ))}
            </div>
          )}

          {isExpanded && (experience.description || highlights.length > 0) && (
            <div className="mt-2.5 pt-2.5 border-t border-white/5 space-y-1.5">
              {experience.description && (
                <p className="text-white/50 text-xs leading-relaxed">{experience.description}</p>
              )}
              {highlights.length > 0 && (
                <ul className="space-y-1">
                  {highlights.map((h, i) => (
                    <li key={i} className="text-white/40 text-xs flex items-start gap-1.5">
                      <span className="text-[#ffd700]/40 mt-0.5 shrink-0">▸</span>
                      {h}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          )}
        </button>
      </div>
    </div>
  );
}

function EmptyState() {
  return (
    <div className="text-center py-16">
      <div className="text-4xl mb-4 opacity-20">◇</div>
      <h3 className="text-white/50 text-sm font-medium mb-2">No experiences yet</h3>
      <p className="text-white/25 text-xs max-w-xs mx-auto leading-relaxed">
        Upload your CV or add experiences manually to see your career timeline come to life.
      </p>
    </div>
  );
}

interface CareerTimelineProps {
  experiences?: ExperienceWithSkills[];
  limit?: number;
  mini?: boolean;
}

export default function CareerTimeline({ experiences: propExperiences, limit, mini }: CareerTimelineProps) {
  const [experiences, setExperiences] = useState<ExperienceWithSkills[] | null>(null);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await listExperiencesWithSkills();
      setExperiences(data);
    } catch {
      setExperiences([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (propExperiences) {
      setExperiences(propExperiences);
      setLoading(false);
    } else {
      load();
    }
  }, [propExperiences, load]);

  const displayed = useMemo(() => {
    const list = experiences ?? [];
    return limit ? list.slice(0, limit) : list;
  }, [experiences, limit]);

  if (loading) {
    return (
      <div className="py-8 text-center text-white/20 text-sm">Loading timeline…</div>
    );
  }

  if (!displayed.length) {
    if (mini) return null;
    return <EmptyState />;
  }

  if (mini) {
    return (
      <div className="space-y-3">
        {displayed.map((exp, i) => (
          <div
            key={exp.id}
            className="flex items-start gap-3 opacity-0"
            style={{ animation: `fadeInUp 0.4s ease-out ${i * 100}ms forwards` }}
          >
            <div className="mt-1.5 w-2.5 h-2.5 rounded-full border border-[#ffd700]/50 bg-[rgba(10,10,10,0.95)] shrink-0" />
            <div className="min-w-0 flex-1">
              <p className="text-white/60 text-xs font-medium truncate">{exp.role}</p>
              <p className="text-white/30 text-[10px]">
                {exp.company} · {formatDate(exp.start_date)} — {formatDate(exp.end_date)}
              </p>
            </div>
          </div>
        ))}
      </div>
    );
  }

  return (
    <div className="relative">
      <div className="absolute left-1/2 -translate-x-px top-0 bottom-0 w-0.5 bg-gradient-to-b from-[#ffd700]/40 via-[#ffd700]/15 to-transparent hidden lg:block" />
      <div className="absolute left-[7px] top-0 bottom-0 w-0.5 bg-gradient-to-b from-[#ffd700]/40 via-[#ffd700]/15 to-transparent lg:hidden" />

      <div className="space-y-8 lg:space-y-10">
        {displayed.map((exp, i) => (
          <TimelineEntry
            key={exp.id}
            experience={exp}
            side={i % 2 === 0 ? "left" : "right"}
            isExpanded={expandedId === exp.id}
            onToggle={() => setExpandedId(expandedId === exp.id ? null : exp.id)}
            animationDelay={i * 120}
          />
        ))}
      </div>
    </div>
  );
}
