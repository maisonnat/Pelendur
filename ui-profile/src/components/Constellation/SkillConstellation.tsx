import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import ForceGraph, { type NodeObject, type ForceGraphMethods } from "react-force-graph-2d";
import { getGraphData, type GraphData, type Skill, type StarStoryRecord, type ExperienceWithSkills } from "../../lib/ipc";

type EntityType = "skill" | "experience" | "star_story" | "company";

interface GraphNode {
  id: string;
  name: string;
  entityType: EntityType;
  category?: string;
  level?: string;
  years?: number;
  usageCount?: number;
  val: number;
  color: string;
}

interface GraphLink {
  source: string;
  target: string;
  relation: string;
  weight: number;
}

const LEVEL_COLORS: Record<string, string> = {
  expert: "#ffd700",
  advanced: "#22c55e",
  intermediate: "#3b82f6",
  learning: "#6b7280",
};

const CATEGORY_COLORS: Record<string, string> = {
  Languages: "#ffd700",
  Frameworks: "#22c55e",
  Tools: "#3b82f6",
  Practices: "#a855f7",
  "Soft Skills": "#f97316",
};

const ENTITY_STYLES: Record<EntityType, { color: string; border: string }> = {
  skill: { color: "#ffd700", border: "#ffd700" },
  experience: { color: "#3b82f6", border: "#3b82f6" },
  star_story: { color: "#f97316", border: "#f97316" },
  company: { color: "#a855f7", border: "#a855f7" },
};

const ENTITY_LABELS: Record<EntityType, string> = {
  skill: "Skills",
  experience: "Experiences",
  star_story: "STAR Stories",
  company: "Companies",
};

function buildGraphData(data: GraphData): { nodes: GraphNode[]; links: GraphLink[] } {
  const nodeMap = new Map<string, GraphNode>();

  const connectionCounts = new Map<string, number>();
  for (const edge of data.edges) {
    connectionCounts.set(edge.source_id, (connectionCounts.get(edge.source_id) || 0) + 1);
    connectionCounts.set(edge.target_id, (connectionCounts.get(edge.target_id) || 0) + 1);
  }

  for (const s of data.skills) {
    const cat = s.category || "Uncategorized";
    const color = LEVEL_COLORS[s.level] || LEVEL_COLORS.learning;
    const connections = connectionCounts.get(s.id) || 0;
    nodeMap.set(s.id, {
      id: s.id,
      name: s.name,
      entityType: "skill",
      category: cat,
      level: s.level,
      years: s.years,
      val: Math.max(4, Math.min(16, 4 + connections * 2)),
      color,
    });
  }

  for (const e of data.experiences) {
    const connections = connectionCounts.get(e.id) || 0;
    nodeMap.set(e.id, {
      id: e.id,
      name: e.company,
      entityType: "experience",
      val: Math.max(5, Math.min(14, 5 + connections)),
      color: ENTITY_STYLES.experience.color,
    });
  }

  for (const s of data.star_stories) {
    const connections = connectionCounts.get(s.id) || 0;
    nodeMap.set(s.id, {
      id: s.id,
      name: s.title || "Untitled Story",
      entityType: "star_story",
      usageCount: s.usage_count,
      val: Math.max(5, Math.min(12, 5 + connections)),
      color: ENTITY_STYLES.star_story.color,
    });
  }

  for (const c of data.companies) {
    const connections = connectionCounts.get(c.id) || 0;
    nodeMap.set(c.id, {
      id: c.id,
      name: c.name,
      entityType: "company",
      val: Math.max(6, Math.min(14, 6 + connections)),
      color: ENTITY_STYLES.company.color,
    });
  }

  const links: GraphLink[] = data.edges
    .filter((e) => nodeMap.has(e.source_id) && nodeMap.has(e.target_id))
    .map((e) => ({
      source: e.source_id,
      target: e.target_id,
      relation: e.relation,
      weight: e.weight,
    }));

  return { nodes: Array.from(nodeMap.values()), links };
}

function paintNode(node: NodeObject<GraphNode>, ctx: CanvasRenderingContext2D, globalScale: number) {
  const n = node as unknown as GraphNode;
  if (!n.name) return;

  const size = (n.val || 5) * 0.5;
  const style = ENTITY_STYLES[n.entityType] || ENTITY_STYLES.skill;

  if (n.entityType === "star_story") {
    drawStar(ctx, n.x || 0, n.y || 0, 5, size, size * 0.45, style.color);
  } else if (n.entityType === "company") {
    drawHexagon(ctx, n.x || 0, n.y || 0, size, style.color);
  } else if (n.entityType === "experience") {
    drawDiamond(ctx, n.x || 0, n.y || 0, size, style.color);
  } else {
    ctx.beginPath();
    ctx.arc(n.x || 0, n.y || 0, size, 0, 2 * Math.PI);
    ctx.fillStyle = style.color + "30";
    ctx.fill();
    ctx.strokeStyle = style.color;
    ctx.lineWidth = 1.5;
    ctx.stroke();
  }

  if (globalScale > 0.6) {
    const fontSize = Math.max(9, 12 / globalScale);
    ctx.font = `${fontSize}px "Segoe UI", Roboto, sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "top";
    ctx.fillStyle = "rgba(255,255,255,0.7)";
    ctx.fillText(n.name, n.x || 0, (n.y || 0) + size + 3);
  }
}

function drawStar(ctx: CanvasRenderingContext2D, cx: number, cy: number, spikes: number, outerR: number, innerR: number, color: string) {
  ctx.beginPath();
  for (let i = 0; i < spikes * 2; i++) {
    const r = i % 2 === 0 ? outerR : innerR;
    const angle = (Math.PI / spikes) * i - Math.PI / 2;
    const x = cx + Math.cos(angle) * r;
    const y = cy + Math.sin(angle) * r;
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.closePath();
  ctx.fillStyle = color + "30";
  ctx.fill();
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.5;
  ctx.stroke();
}

function drawHexagon(ctx: CanvasRenderingContext2D, cx: number, cy: number, r: number, color: string) {
  ctx.beginPath();
  for (let i = 0; i < 6; i++) {
    const angle = (Math.PI / 3) * i - Math.PI / 6;
    const x = cx + Math.cos(angle) * r;
    const y = cy + Math.sin(angle) * r;
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.closePath();
  ctx.fillStyle = color + "30";
  ctx.fill();
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.5;
  ctx.stroke();
}

function drawDiamond(ctx: CanvasRenderingContext2D, cx: number, cy: number, r: number, color: string) {
  ctx.beginPath();
  ctx.moveTo(cx, cy - r);
  ctx.lineTo(cx + r * 0.7, cy);
  ctx.lineTo(cx, cy + r);
  ctx.lineTo(cx - r * 0.7, cy);
  ctx.closePath();
  ctx.fillStyle = color + "30";
  ctx.fill();
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.5;
  ctx.stroke();
}

function nodePointerArea(node: NodeObject<GraphNode>, color: string, ctx: CanvasRenderingContext2D) {
  const n = node as unknown as GraphNode;
  const size = (n.val || 5) * 0.5;
  ctx.fillStyle = color;
  ctx.beginPath();
  ctx.arc(n.x || 0, n.y || 0, size + 2, 0, 2 * Math.PI);
  ctx.fill();
}

type FilterState = {
  types: Set<EntityType>;
  categories: Set<string>;
  search: string;
};

function DetailPanel({
  node,
  stories,
  skills,
  experiences,
  onClose,
}: {
  node: GraphNode | null;
  stories: StarStoryRecord[];
  skills: Skill[];
  experiences: ExperienceWithSkills[];
  onClose: () => void;
}) {
  if (!node) return null;

  return (
    <div className="absolute top-0 right-0 w-80 h-full bg-[rgba(15,15,15,0.92)] backdrop-blur-xl border-l border-white/5 z-10 flex flex-col">
      <div className="flex items-center justify-between px-4 py-3 border-b border-white/5">
        <h3 className="text-sm font-medium text-[#ffd700]">
          {node.name}
        </h3>
        <button
          onClick={onClose}
          className="text-white/30 hover:text-white/60 text-xs transition-colors"
        >
          ✕
        </button>
      </div>
      <div className="flex-1 overflow-auto p-4 space-y-3">
        <div className="flex items-center gap-2">
          <span
            className="text-[10px] px-2 py-0.5 rounded uppercase tracking-wider font-medium"
            style={{
              color: ENTITY_STYLES[node.entityType].color,
              backgroundColor: ENTITY_STYLES[node.entityType].color + "18",
            }}
          >
            {ENTITY_LABELS[node.entityType]}
          </span>
        </div>

        {node.entityType === "skill" && (
          <>
            {node.level && (
              <div>
                <span className="text-[10px] text-white/25 uppercase tracking-wider">Level</span>
                <div className="mt-1 flex items-center gap-2">
                  <span
                    className="text-xs font-medium px-2 py-0.5 rounded"
                    style={{ color: LEVEL_COLORS[node.level] || "#6b7280", backgroundColor: (LEVEL_COLORS[node.level] || "#6b7280") + "18" }}
                  >
                    {node.level}
                  </span>
                  <span className="text-xs text-white/30">{node.years} yrs</span>
                </div>
              </div>
            )}
            {node.category && (
              <div>
                <span className="text-[10px] text-white/25 uppercase tracking-wider">Category</span>
                <p className="text-xs text-white/50 mt-1">{node.category}</p>
              </div>
            )}
          </>
        )}

        {node.entityType === "star_story" && (() => {
          const story = stories.find((s) => s.id === node.id);
          if (!story) return null;
          return (
            <div className="space-y-3">
              <div>
                <span className="text-[10px] text-white/25 uppercase tracking-wider">Situation</span>
                <p className="text-xs text-white/60 mt-1">{story.situation}</p>
              </div>
              <div>
                <span className="text-[10px] text-white/25 uppercase tracking-wider">Task</span>
                <p className="text-xs text-white/60 mt-1">{story.task}</p>
              </div>
              <div>
                <span className="text-[10px] text-white/25 uppercase tracking-wider">Action</span>
                <p className="text-xs text-white/60 mt-1">{story.action}</p>
              </div>
              <div>
                <span className="text-[10px] text-white/25 uppercase tracking-wider">Result</span>
                <p className="text-xs text-white/60 mt-1">{story.result}</p>
              </div>
              {story.tags && (
                <div>
                  <span className="text-[10px] text-white/25 uppercase tracking-wider">Tags</span>
                  <div className="flex flex-wrap gap-1 mt-1">
                    {JSON.parse(story.tags).map((t: string) => (
                      <span key={t} className="text-[10px] px-1.5 py-0.5 rounded bg-white/5 text-white/40">
                        {t}
                      </span>
                    ))}
                  </div>
                </div>
              )}
              {story.usage_count > 0 && (
                <div className="text-[11px] text-white/20">
                  Practiced {story.usage_count} time{story.usage_count !== 1 ? "s" : ""}
                </div>
              )}
            </div>
          );
        })()}

        {node.entityType === "experience" && (() => {
          const exp = experiences.find((e) => e.id === node.id);
          if (!exp) return null;
          return (
            <div className="space-y-3">
              <div>
                <span className="text-[10px] text-white/25 uppercase tracking-wider">Role</span>
                <p className="text-xs text-white/60 mt-1">{exp.role}</p>
              </div>
              <div>
                <span className="text-[10px] text-white/25 uppercase tracking-wider">Period</span>
                <p className="text-xs text-white/60 mt-1">
                  {exp.start_date} → {exp.end_date || "Present"}
                </p>
              </div>
              {exp.description && (
                <div>
                  <span className="text-[10px] text-white/25 uppercase tracking-wider">Description</span>
                  <p className="text-xs text-white/60 mt-1">{exp.description}</p>
                </div>
              )}
              {exp.skills.length > 0 && (
                <div>
                  <span className="text-[10px] text-white/25 uppercase tracking-wider">Skills</span>
                  <div className="flex flex-wrap gap-1 mt-1">
                    {exp.skills.map((s) => (
                      <span key={s} className="text-[10px] px-1.5 py-0.5 rounded bg-[#ffd700]/10 text-[#ffd700]/70">
                        {s}
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>
          );
        })()}
      </div>
    </div>
  );
}

export default function SkillConstellation() {
  const [rawData, setRawData] = useState<GraphData | null>(null);
  const [loading, setLoading] = useState(true);
  const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null);
  const [hoverNode, setHoverNode] = useState<GraphNode | null>(null);
  const [filters, setFilters] = useState<FilterState>({
    types: new Set(["skill", "experience", "star_story", "company"] as EntityType[]),
    categories: new Set<string>(),
    search: "",
  });
  const fgRef = useRef<ForceGraphMethods | undefined>(undefined);
  const containerRef = useRef<HTMLDivElement>(null);
  const [dims, setDims] = useState({ width: 800, height: 600 });

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const d = await getGraphData();
      setRawData(d);
    } catch {
      setRawData(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const obs = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        if (width > 0 && height > 0) setDims({ width, height });
      }
    });
    obs.observe(el);
    return () => obs.disconnect();
  }, []);

  const allCategories = useMemo(() => {
    if (!rawData) return [];
    const cats = new Set<string>();
    for (const s of rawData.skills) {
      if (s.category) cats.add(s.category);
    }
    return Array.from(cats).sort();
  }, [rawData]);

  const graphData = useMemo(() => {
    if (!rawData) return { nodes: [], links: [] };
    const { nodes, links } = buildGraphData(rawData);

    const filtered = nodes.filter((n) => {
      if (!filters.types.has(n.entityType)) return false;
      if (n.entityType === "skill" && filters.categories.size > 0) {
        if (!n.category || !filters.categories.has(n.category)) return false;
      }
      if (filters.search) {
        const q = filters.search.toLowerCase();
        if (!n.name.toLowerCase().includes(q)) return false;
      }
      return true;
    });

    const filteredIds = new Set(filtered.map((n) => n.id));
    const filteredLinks = links.filter(
      (l) => filteredIds.has(String(l.source)) && filteredIds.has(String(l.target))
    );

    return { nodes: filtered, links: filteredLinks };
  }, [rawData, filters]);

  const totalNodes = rawData
    ? rawData.skills.length + rawData.experiences.length + rawData.star_stories.length + rawData.companies.length
    : 0;

  const toggleType = (type: EntityType) => {
    setFilters((prev) => {
      const next = new Set(prev.types);
      if (next.has(type)) next.delete(type);
      else next.add(type);
      return { ...prev, types: next };
    });
  };

  const toggleCategory = (cat: string) => {
    setFilters((prev) => {
      const next = new Set(prev.categories);
      if (next.has(cat)) next.delete(cat);
      else next.add(cat);
      return { ...prev, categories: next };
    });
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-white/20 text-sm">Loading constellation…</div>
      </div>
    );
  }

  if (!rawData || totalNodes === 0) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="bg-[rgba(30,30,30,0.85)] backdrop-blur-md rounded-xl border border-white/5 p-8 text-center max-w-sm">
          <div className="text-3xl mb-3">🌌</div>
          <h3 className="text-sm font-medium text-[#ffd700] mb-2">Your Skill Constellation Awaits</h3>
          <p className="text-xs text-white/30">
            Upload your CV to see your skill constellation — an interactive map of your skills, experiences, and stories.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-[calc(100vh-7rem)] -m-6" ref={containerRef}>
      <div className="flex-1 relative">
        <ForceGraph
          ref={(el) => { fgRef.current = el; }}
          graphData={graphData}
          width={dims.width - (selectedNode ? 320 : 0)}
          height={dims.height}
          nodeId="id"
          linkSource="source"
          linkTarget="target"
          backgroundColor="transparent"
          nodeLabel={(n) => {
            const gn = n as unknown as GraphNode;
            const label = `${gn.name} (${ENTITY_LABELS[gn.entityType]?.replace(/s$/, "") || gn.entityType})`;
            if (gn.entityType === "skill" && gn.level) return `${label} — ${gn.level}`;
            return label;
          }}
          nodeVal="val"
          nodeCanvasObjectMode={() => "replace"}
          nodeCanvasObject={paintNode}
          nodePointerAreaPaint={nodePointerArea}
          onNodeClick={(node) => {
            setSelectedNode(node as unknown as GraphNode);
          }}
          onNodeHover={(node) => {
            setHoverNode(node as unknown as GraphNode | null);
          }}
          onBackgroundClick={() => setSelectedNode(null)}
          linkColor={(link) => {
            const l = link as unknown as GraphLink;
            if (hoverNode) {
              const src = String(l.source);
              const tgt = String(l.target);
              if (src === hoverNode.id || tgt === hoverNode.id) {
                return "rgba(255,215,0,0.5)";
              }
              return "rgba(255,255,255,0.03)";
            }
            return "rgba(255,255,255,0.07)";
          }}
          linkWidth={(link) => {
            const l = link as unknown as GraphLink;
            return Math.max(0.5, l.weight * 1.2);
          }}
          linkVisibility={true}
          linkDirectionalArrowLength={3}
          linkDirectionalArrowColor="rgba(255,255,255,0.12)"
          linkDirectionalArrowRelPos={0.9}
          enableNodeDrag={true}
          enableZoomInteraction={true}
          enablePanInteraction={true}
          minZoom={0.2}
          maxZoom={5}
          warmupTicks={50}
          cooldownTicks={150}
          d3AlphaDecay={0.02}
          d3VelocityDecay={0.3}
        />

        <div className="absolute top-3 left-3 bg-[rgba(15,15,15,0.85)] backdrop-blur-md rounded-lg border border-white/5 px-3 py-2 flex items-center gap-4 text-[10px] text-white/30">
          <span>{graphData.nodes.length} nodes</span>
          <span>{graphData.links.length} edges</span>
          <span>Scroll to zoom · Drag to pan</span>
        </div>

        <div className="absolute bottom-3 left-3 bg-[rgba(15,15,15,0.85)] backdrop-blur-md rounded-lg border border-white/5 px-3 py-2">
          <div className="flex items-center gap-3 text-[10px]">
            {(Object.entries(ENTITY_LABELS) as [EntityType, string][]).map(([type, label]) => (
              <button
                key={type}
                onClick={() => toggleType(type)}
                className={`flex items-center gap-1.5 transition-opacity ${filters.types.has(type) ? "opacity-100" : "opacity-30"}`}
              >
                <span
                  className="w-2 h-2 rounded-full"
                  style={{ backgroundColor: ENTITY_STYLES[type].color }}
                />
                <span className="text-white/50">{label}</span>
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="w-56 bg-[rgba(15,15,15,0.85)] backdrop-blur-md border-l border-white/5 flex flex-col">
        <div className="px-3 py-3 border-b border-white/5">
          <h3 className="text-[11px] text-white/30 uppercase tracking-wider font-medium">Filters</h3>
        </div>

        <div className="px-3 py-3 border-b border-white/5">
          <input
            type="text"
            value={filters.search}
            onChange={(e) => setFilters((f) => ({ ...f, search: e.target.value }))}
            placeholder="Search nodes…"
            className="w-full bg-white/[0.04] border border-white/[0.06] rounded-md px-2.5 py-1.5 text-xs text-white/70 placeholder-white/20 focus:outline-none focus:border-[#ffd700]/30"
          />
        </div>

        <div className="px-3 py-3 border-b border-white/5">
          <h4 className="text-[10px] text-white/20 uppercase tracking-wider mb-2">Skill Categories</h4>
          <div className="space-y-1">
            {allCategories.map((cat) => (
              <button
                key={cat}
                onClick={() => toggleCategory(cat)}
                className={`flex items-center gap-2 w-full text-left text-[11px] transition-opacity ${
                  filters.categories.size === 0 || filters.categories.has(cat) ? "opacity-100" : "opacity-30"
                }`}
              >
                <span
                  className="w-2 h-2 rounded-full shrink-0"
                  style={{ backgroundColor: CATEGORY_COLORS[cat] || "#6b7280" }}
                />
                <span className="text-white/50">{cat}</span>
              </button>
            ))}
          </div>
        </div>

        <div className="px-3 py-3 flex-1">
          <h4 className="text-[10px] text-white/20 uppercase tracking-wider mb-2">Legend</h4>
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <svg width="14" height="14" viewBox="0 0 14 14">
                <circle cx="7" cy="7" r="5" fill="rgba(255,215,0,0.2)" stroke="#ffd700" strokeWidth="1.5" />
              </svg>
              <span className="text-[10px] text-white/30">Skill (colored by level)</span>
            </div>
            <div className="flex items-center gap-2">
              <svg width="14" height="14" viewBox="0 0 14 14">
                <polygon points="7,1 12,7 7,13 2,7" fill="rgba(59,130,246,0.2)" stroke="#3b82f6" strokeWidth="1.5" />
              </svg>
              <span className="text-[10px] text-white/30">Experience</span>
            </div>
            <div className="flex items-center gap-2">
              <svg width="14" height="14" viewBox="0 0 14 14">
                <polygon points="7,1 9,5 13,5 10,8 11,13 7,10 3,13 4,8 1,5 5,5" fill="rgba(249,115,22,0.2)" stroke="#f97316" strokeWidth="1.5" />
              </svg>
              <span className="text-[10px] text-white/30">STAR Story</span>
            </div>
            <div className="flex items-center gap-2">
              <svg width="14" height="14" viewBox="0 0 14 14">
                <polygon points="7,1 12.2,4 12.2,10 7,13 1.8,10 1.8,4" fill="rgba(168,85,247,0.2)" stroke="#a855f7" strokeWidth="1.5" />
              </svg>
              <span className="text-[10px] text-white/30">Company</span>
            </div>
          </div>
        </div>

        <div className="px-3 py-3 border-t border-white/5 space-y-1.5">
          <div className="flex items-center gap-1.5">
            <span className="w-3 h-0.5 bg-white/10 rounded" />
            <span className="text-[9px] text-white/20">Weak relation</span>
          </div>
          <div className="flex items-center gap-1.5">
            <span className="w-3 h-1 bg-white/30 rounded" />
            <span className="text-[9px] text-white/20">Strong relation</span>
          </div>
        </div>
      </div>

      {selectedNode && (
        <DetailPanel
          node={selectedNode}
          stories={rawData.star_stories}
          skills={rawData.skills}
          experiences={rawData.experiences}
          onClose={() => setSelectedNode(null)}
        />
      )}
    </div>
  );
}
