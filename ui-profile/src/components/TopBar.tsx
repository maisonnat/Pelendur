import { useState, useEffect, useCallback } from "react";
import { fetchKnowledgeGraphStats, type KnowledgeGraphStats } from "../lib/ipc";

export default function TopBar() {
  const [stats, setStats] = useState<KnowledgeGraphStats | null>(null);
  const [searchQuery, setSearchQuery] = useState("");

  const loadStats = useCallback(async () => {
    try {
      const s = await fetchKnowledgeGraphStats();
      setStats(s);
    } catch {
      setStats(null);
    }
  }, []);

  useEffect(() => {
    loadStats();
  }, [loadStats]);

  return (
    <header className="h-12 shrink-0 flex items-center justify-between px-6 bg-[rgba(20,20,20,0.9)] border-b border-white/5">
      <div className="flex items-center gap-6">
        {stats && (
          <div className="flex items-center gap-4 text-xs text-white/30">
            <span>
              <span className="text-[#ffd700]/70 font-medium">{stats.skills}</span> skills
            </span>
            <span className="text-white/10">│</span>
            <span>
              <span className="text-[#ffd700]/70 font-medium">{stats.star_stories}</span> stories
            </span>
            <span className="text-white/10">│</span>
            <span>
              <span className="text-[#ffd700]/70 font-medium">{stats.experiences}</span> experiences
            </span>
            <span className="text-white/10">│</span>
            <span>
              <span className="text-[#ffd700]/70 font-medium">{stats.companies}</span> companies
            </span>
          </div>
        )}
      </div>

      <div className="relative w-64">
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="Search knowledge graph…"
          className="w-full bg-white/[0.04] border border-white/[0.06] rounded-md px-3 py-1.5 text-xs text-white/60 placeholder-white/20 focus:outline-none focus:border-[#ffd700]/30 focus:bg-white/[0.06] transition-colors"
        />
        <span className="absolute right-2.5 top-1/2 -translate-y-1/2 text-white/15 text-xs">
          ⌘K
        </span>
      </div>
    </header>
  );
}
