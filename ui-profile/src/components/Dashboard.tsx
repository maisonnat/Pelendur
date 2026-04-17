import { useState, useEffect, useCallback } from "react";
import { fetchKnowledgeGraphStats, type KnowledgeGraphStats } from "../lib/ipc";
import CareerTimeline from "./Timeline/CareerTimeline";

function CircularProgress({ percent }: { percent: number }) {
  const radius = 54;
  const stroke = 6;
  const normalizedRadius = radius - stroke / 2;
  const circumference = normalizedRadius * 2 * Math.PI;
  const offset = circumference - (percent / 100) * circumference;

  return (
    <div className="relative inline-flex items-center justify-center">
      <svg height={radius * 2} width={radius * 2} className="-rotate-90">
        <circle
          stroke="rgba(255,255,255,0.06)"
          fill="transparent"
          strokeWidth={stroke}
          r={normalizedRadius}
          cx={radius}
          cy={radius}
        />
        <circle
          stroke="#ffd700"
          fill="transparent"
          strokeWidth={stroke}
          strokeDasharray={`${circumference} ${circumference}`}
          style={{ strokeDashoffset: offset, transition: "stroke-dashoffset 0.8s ease-out" }}
          strokeLinecap="round"
          r={normalizedRadius}
          cx={radius}
          cy={radius}
        />
      </svg>
      <div className="absolute inset-0 flex flex-col items-center justify-center">
        <span className="text-2xl font-semibold text-[#ffd700]">{percent}%</span>
        <span className="text-[10px] text-white/30 uppercase tracking-widest mt-0.5">Complete</span>
      </div>
    </div>
  );
}

function StatCard({ label, value, accent }: { label: string; value: number; accent?: boolean }) {
  return (
    <div className="bg-[rgba(30,30,30,0.85)] backdrop-blur-md rounded-lg border border-white/5 p-4 flex flex-col gap-1 min-w-[120px]">
      <span className={`text-2xl font-semibold ${accent ? "text-[#ffd700]" : "text-white/80"}`}>
        {value}
      </span>
      <span className="text-[11px] text-white/30 uppercase tracking-wider">{label}</span>
    </div>
  );
}

function computeCompleteness(stats: KnowledgeGraphStats): number {
  const weights = [
    { value: stats.skills, target: 10, weight: 25 },
    { value: stats.star_stories, target: 5, weight: 25 },
    { value: stats.experiences, target: 5, weight: 25 },
    { value: stats.companies, target: 3, weight: 25 },
  ];
  const score = weights.reduce((acc, w) => {
    return acc + Math.min(w.value / w.target, 1) * w.weight;
  }, 0);
  return Math.round(score);
}

const RECENT_ACTIVITY = [
  { type: "story", text: "Added STAR story: \"Led migration to microservices\"", time: "2h ago" },
  { type: "skill", text: "Updated skill: React → Advanced", time: "1d ago" },
  { type: "meeting", text: "Captured learnings from: Tech Review Session", time: "2d ago" },
  { type: "company", text: "Added company: Stripe", time: "3d ago" },
];

const ACTIVITY_ICONS: Record<string, string> = {
  story: "★",
  skill: "⬡",
  meeting: "◈",
  company: "⌘",
};

export default function Dashboard() {
  const [stats, setStats] = useState<KnowledgeGraphStats | null>(null);
  const [loading, setLoading] = useState(true);

  const loadStats = useCallback(async () => {
    setLoading(true);
    try {
      const s = await fetchKnowledgeGraphStats();
      setStats(s);
    } catch {
      setStats(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadStats();
  }, [loadStats]);

  const completeness = stats ? computeCompleteness(stats) : 0;

  return (
    <div className="space-y-6 max-w-4xl">
      <div className="flex items-start gap-8">
        <div className="bg-[rgba(30,30,30,0.85)] backdrop-blur-md rounded-xl border border-white/5 p-6 flex flex-col items-center gap-3">
          <CircularProgress percent={completeness} />
          <div className="text-center">
            <h2 className="text-sm font-medium text-white/70">Profile Strength</h2>
            <p className="text-[11px] text-white/25 mt-1">
              {completeness >= 80
                ? "All-Star profile! You're interview-ready."
                : completeness >= 50
                  ? "Good progress. Add more stories to stand out."
                  : "Getting started. Fill in your skills and experiences."}
            </p>
      </div>

      <div className="bg-[rgba(30,30,30,0.85)] backdrop-blur-md rounded-xl border border-white/5 p-6">
        <h3 className="text-sm font-medium text-white/50 uppercase tracking-wider mb-4">
          Recent Experience
        </h3>
        <CareerTimeline limit={3} mini />
      </div>
    </div>

        <div className="flex-1 grid grid-cols-2 gap-3">
          {loading ? (
            <div className="col-span-2 text-white/20 text-sm py-8 text-center">Loading stats…</div>
          ) : stats ? (
            <>
              <StatCard label="Skills" value={stats.skills} accent />
              <StatCard label="STAR Stories" value={stats.star_stories} />
              <StatCard label="Experiences" value={stats.experiences} />
              <StatCard label="Companies" value={stats.companies} />
            </>
          ) : (
            <div className="col-span-2 text-red-400/60 text-sm py-8 text-center">
              Failed to load stats
            </div>
          )}
        </div>
      </div>

      <div className="bg-[rgba(30,30,30,0.85)] backdrop-blur-md rounded-xl border border-white/5 p-6">
        <h3 className="text-sm font-medium text-white/50 uppercase tracking-wider mb-4">
          Recent Activity
        </h3>
        <div className="space-y-0">
          {RECENT_ACTIVITY.map((item, i) => (
            <div
              key={i}
              className="flex items-start gap-3 py-3 border-b border-white/[0.03] last:border-0"
            >
              <span className="text-[#ffd700]/50 text-sm mt-0.5">
                {ACTIVITY_ICONS[item.type] ?? "•"}
              </span>
              <div className="flex-1 min-w-0">
                <p className="text-sm text-white/60 truncate">{item.text}</p>
              </div>
              <span className="text-[11px] text-white/15 shrink-0">{item.time}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
