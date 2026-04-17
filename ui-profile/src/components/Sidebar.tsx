import { NavLink } from "react-router-dom";

const NAV_ITEMS = [
  { to: "/", label: "Dashboard", icon: "◈" },
  { to: "/skills", label: "Skills", icon: "⬡" },
  { to: "/experiences", label: "Experiences", icon: "◇" },
  { to: "/star-stories", label: "STAR Stories", icon: "★" },
  { to: "/constellation", label: "Constellation", icon: "✦" },
  { to: "/companies", label: "Companies", icon: "⌘" },
  { to: "/practice", label: "Practice", icon: "🎯" },
  { to: "/settings", label: "Settings", icon: "⚙" },
  { to: "/debrief", label: "Debrief", icon: "📋" },
] as const;

export default function Sidebar() {
  return (
    <nav className="w-56 shrink-0 bg-[rgba(30,30,30,0.85)] backdrop-blur-md border-r border-white/5 flex flex-col">
      <div className="px-5 pt-6 pb-4">
        <h1 className="text-[#ffd700] text-lg font-semibold tracking-wide">
          Pelendur
        </h1>
        <p className="text-white/30 text-[11px] mt-0.5 tracking-widest uppercase">
          Profile Manager
        </p>
      </div>

      <div className="flex-1 px-3 space-y-0.5">
        {NAV_ITEMS.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.to === "/"}
            className={({ isActive }) =>
              [
                "flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm transition-all duration-150",
                isActive
                  ? "bg-[rgba(255,215,0,0.1)] text-[#ffd700] border border-[rgba(255,215,0,0.15)]"
                  : "text-white/40 hover:text-white/70 hover:bg-white/[0.03] border border-transparent",
              ].join(" ")
            }
          >
            <span className="text-base leading-none">{item.icon}</span>
            <span>{item.label}</span>
          </NavLink>
        ))}
      </div>

      <div className="px-5 py-4 border-t border-white/5">
        <div className="flex items-center gap-2">
          <div className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
          <span className="text-white/25 text-xs">Knowledge Graph Active</span>
        </div>
      </div>
    </nav>
  );
}
