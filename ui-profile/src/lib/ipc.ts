/**
 * Tauri IPC integration layer.
 * Wraps invoke() calls with typed interfaces and graceful fallback
 * when running outside Tauri (browser dev mode).
 * 
 * For dev mode without Tauri runtime, we mock the API.
 */

// Mock invoke function for development
const mockInvoke = async (cmd: string, args?: Record<string, unknown>): Promise<unknown> => {
  console.warn(`[IPC Mock] ${cmd}`, args);
  
  // Return appropriate mock data based on command
  switch (cmd) {
    case 'get_knowledge_graph_stats':
      return { skills: 12, experiences: 8, star_stories: 5, projects: 14, companies: 3 };
    case 'search_knowledge':
    case 'search_knowledge_enhanced':
      return [{ provider: 'mock', content: `No results (dev mode)` }];
    case 'get_star_stories':
      return getMockStarStories();
    case 'get_graph_data':
      return getMockGraphData();
    case 'list_skills':
      return getMockSkills();
    case 'list_experiences_with_skills':
      return getMockExperiences();
    case 'list_companies':
      return getMockCompanies();
    case 'list_all_edges':
      return getMockEdges();
    default:
      return null;
  }
};

// ── Types ──────────────────────────────────────────────────────────────

interface KnowledgeGraphStats {
  skills: number;
  experiences: number;
  star_stories: number;
  projects: number;
  companies: number;
}

interface SearchResult {
  provider: string;
  content: string;
}

export interface StarStoryData {
  id?: string;
  title?: string;
  situation: string;
  task: string;
  action: string;
  result: string;
  tags?: string;
  difficulty?: string;
  stakes?: string;
}

export interface StarStoryRecord {
  id: string;
  title: string | null;
  situation: string;
  task: string;
  action: string;
  result: string;
  tags: string | null;
  difficulty: string | null;
  stakes: string | null;
  usage_count: number;
  created_at: string;
  updated_at: string;
}

// ── Invoke wrapper ────────────────────────────────────────────────────

let invokeFn: ((cmd: string, args?: Record<string, unknown>) => Promise<unknown>) | null = null;

async function getInvoke() {
  if (invokeFn) return invokeFn;
  
  try {
    // Try to dynamically import Tauri API
    const tauriModule = await import('@tauri-apps/api/tauri');
    invokeFn = tauriModule.invoke as (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
    console.log('[IPC] Tauri runtime detected');
    return invokeFn;
  } catch {
    console.warn('[IPC] Running outside Tauri — using mock data');
    invokeFn = mockInvoke;
    return invokeFn;
  }
}

// ── Mock Data Helpers ────────────────────────────────────────────────

function getMockStarStories(): StarStoryRecord[] {
  return [
    {
      id: 'mock-1',
      title: 'Leadership in Crisis',
      situation: 'Team lost the tech lead during a critical sprint',
      task: 'Had to take ownership without formal title',
      action: 'Organized standups, created pair programming rotations',
      result: 'Delivered 2 weeks early, 0 bugs in production',
      tags: '["leadership","crisis","team-management"]',
      difficulty: 'medium',
      stakes: 'high',
      usage_count: 3,
      created_at: '2026-04-10T10:00:00Z',
      updated_at: '2026-04-15T14:30:00Z',
    },
    {
      id: 'mock-2',
      title: 'API Performance Optimization',
      situation: '2s latency on main payment API endpoint',
      task: 'Reduce to <200ms without architecture changes',
      action: 'Implemented Redis caching, optimized queries, added indexes',
      result: '180ms p99 latency, 40% cost reduction, 15% conversion increase',
      tags: '["performance","backend","optimization"]',
      difficulty: 'hard',
      stakes: 'high',
      usage_count: 5,
      created_at: '2026-04-08T08:00:00Z',
      updated_at: '2026-04-14T12:00:00Z',
    },
    {
      id: 'mock-3',
      title: 'Cross-Team Migration',
      situation: 'Legacy monolith blocking feature velocity',
      task: 'Migrate to microservices across 3 teams',
      action: 'Designed event-driven architecture, led weekly syncs',
      result: 'Deploy frequency up 10x, incidents down 70%',
      tags: '["architecture","migration","leadership"]',
      difficulty: 'hard',
      stakes: 'high',
      usage_count: 2,
      created_at: '2026-04-05T09:00:00Z',
      updated_at: '2026-04-12T16:00:00Z',
    },
  ];
}

export interface ExperienceWithSkills {
  id: string;
  company: string;
  role: string;
  start_date: string;
  end_date: string | null;
  description: string | null;
  highlights: string | null;
  skills: string[];
}

export interface Skill {
  id: string;
  name: string;
  category: string | null;
  level: string;
  years: number;
  source: string | null;
  created_at: string;
  updated_at: string;
}

export interface Edge {
  id: string;
  source_id: string;
  source_type: string;
  target_id: string;
  target_type: string;
  relation: string;
  weight: number;
}

function getMockSkills(): Skill[] {
  return [
    { id: '1', name: 'TypeScript', category: 'Languages', level: 'expert', years: 6, source: null, created_at: '', updated_at: '' },
    { id: '2', name: 'Rust', category: 'Languages', level: 'advanced', years: 3, source: null, created_at: '', updated_at: '' },
    { id: '3', name: 'React', category: 'Frameworks', level: 'expert', years: 5, source: null, created_at: '', updated_at: '' },
    { id: '4', name: 'Docker', category: 'Tools', level: 'intermediate', years: 4, source: null, created_at: '', updated_at: '' },
    { id: '5', name: 'System Design', category: 'Practices', level: 'advanced', years: 5, source: null, created_at: '', updated_at: '' },
    { id: '6', name: 'Leadership', category: 'Soft Skills', level: 'learning', years: 2, source: null, created_at: '', updated_at: '' },
    { id: '7', name: 'Go', category: 'Languages', level: 'expert', years: 5, source: null, created_at: '', updated_at: '' },
    { id: '8', name: 'PostgreSQL', category: 'Tools', level: 'advanced', years: 4, source: null, created_at: '', updated_at: '' },
    { id: '9', name: 'Kubernetes', category: 'Tools', level: 'intermediate', years: 3, source: null, created_at: '', updated_at: '' },
    { id: '10', name: 'Redis', category: 'Tools', level: 'advanced', years: 3, source: null, created_at: '', updated_at: '' },
  ];
}

function getMockExperiences(): ExperienceWithSkills[] {
  return [
    {
      id: 'mock-1',
      company: 'Stripe',
      role: 'Senior Backend Engineer',
      start_date: '2022-03',
      end_date: null,
      description: 'Building payment infrastructure for global merchants.',
      highlights: '["Led migration to microservices","Reduced API latency by 60%"]',
      skills: ['Go', 'Kubernetes', 'PostgreSQL'],
    },
    {
      id: 'mock-2',
      company: 'Acme Corp',
      role: 'Backend Engineer',
      start_date: '2019-06',
      end_date: '2022-02',
      description: 'API development and system optimization.',
      highlights: '["Built high-performance API gateway","Designed caching strategy"]',
      skills: ['Go', 'Redis'],
    },
    {
      id: 'mock-3',
      company: 'StartupXYZ',
      role: 'Junior Developer',
      start_date: '2017-01',
      end_date: '2019-05',
      description: 'Full-stack development for early-stage startup.',
      highlights: null,
      skills: ['React', 'PostgreSQL'],
    },
  ];
}

function getMockCompanies() {
  return [
    { id: 'comp-1', name: 'Stripe', industry: 'Fintech', description: 'Payment infrastructure', tech_stack: 'Go, Kubernetes, PostgreSQL' },
    { id: 'comp-2', name: 'Acme Corp', industry: 'SaaS', description: 'API platform', tech_stack: 'Go, Redis, gRPC' },
    { id: 'comp-3', name: 'StartupXYZ', industry: 'Technology', description: 'Developer tools', tech_stack: 'React, PostgreSQL' },
  ];
}

function getMockEdges(): Edge[] {
  return [
    { id: 'e1', source_id: '7', source_type: 'skill', target_id: 'mock-1', target_type: 'experience', relation: 'used_in', weight: 1.0 },
    { id: 'e2', source_id: '9', source_type: 'skill', target_id: 'mock-1', target_type: 'experience', relation: 'used_in', weight: 1.0 },
    { id: 'e3', source_id: '10', source_type: 'skill', target_id: 'mock-2', target_type: 'experience', relation: 'used_in', weight: 1.0 },
    { id: 'e4', source_id: '3', source_type: 'skill', target_id: 'mock-3', target_type: 'experience', relation: 'used_in', weight: 1.0 },
  ];
}

export interface GraphData {
  skills: Skill[];
  experiences: ExperienceWithSkills[];
  star_stories: StarStoryRecord[];
  companies: Array<{ id: string; name: string; industry: string | null; description: string | null }>;
  edges: Edge[];
}

function getMockGraphData(): GraphData {
  return {
    skills: getMockSkills(),
    experiences: getMockExperiences(),
    star_stories: getMockStarStories(),
    companies: getMockCompanies(),
    edges: getMockEdges(),
  };
}

// ── Public API ────────────────────────────────────────────────────────

export async function fetchKnowledgeGraphStats(): Promise<KnowledgeGraphStats> {
  const fn = await getInvoke();
  return fn('get_knowledge_graph_stats') as Promise<KnowledgeGraphStats>;
}

export async function searchKnowledge(query: string): Promise<SearchResult[]> {
  const fn = await getInvoke();
  return fn('search_knowledge', { query }) as Promise<SearchResult[]>;
}

export async function createStarStory(data: StarStoryData): Promise<StarStoryRecord> {
  const fn = await getInvoke();
  return fn('create_star_story', { data }) as Promise<StarStoryRecord>;
}

export async function updateStarStory(data: StarStoryData): Promise<StarStoryRecord> {
  const fn = await getInvoke();
  return fn('update_star_story', { data }) as Promise<StarStoryRecord>;
}

export async function deleteStarStory(id: string): Promise<boolean> {
  const fn = await getInvoke();
  return fn('delete_star_story', { id }) as Promise<boolean>;
}

export async function getStarStories(): Promise<StarStoryRecord[]> {
  const fn = await getInvoke();
  return fn('get_star_stories') as Promise<StarStoryRecord[]>;
}

export async function coachStarStory(storyId: string | null, question: string): Promise<string> {
  const fn = await getInvoke();
  return fn('coach_star_story', { storyId, question }) as Promise<string>;
}

export async function listExperiencesWithSkills(): Promise<ExperienceWithSkills[]> {
  const fn = await getInvoke();
  return fn('list_experiences_with_skills') as Promise<ExperienceWithSkills[]>;
}

export async function listSkills(): Promise<Skill[]> {
  const fn = await getInvoke();
  return fn('list_skills') as Promise<Skill[]>;
}

export async function createSkill(data: { name: string; category?: string; level: string; years: number }): Promise<Skill> {
  const fn = await getInvoke();
  return fn('create_skill', { data }) as Promise<Skill>;
}

export async function updateSkill(data: { id: string; name: string; category?: string; level: string; years: number }): Promise<Skill> {
  const fn = await getInvoke();
  return fn('update_skill', { data }) as Promise<Skill>;
}

export async function deleteSkill(id: string): Promise<boolean> {
  const fn = await getInvoke();
  return fn('delete_skill', { id }) as Promise<boolean>;
}

export async function createExperience(data: {
  company: string; role: string; start_date: string; end_date?: string;
  description?: string; highlights?: string; skill_ids?: string[];
}): Promise<ExperienceWithSkills> {
  const fn = await getInvoke();
  return fn('create_experience', { data }) as Promise<ExperienceWithSkills>;
}

export async function updateExperience(data: {
  id: string; company: string; role: string; start_date: string; end_date?: string;
  description?: string; highlights?: string; skill_ids?: string[];
}): Promise<ExperienceWithSkills> {
  const fn = await getInvoke();
  return fn('update_experience', { data }) as Promise<ExperienceWithSkills>;
}

export async function deleteExperience(id: string): Promise<boolean> {
  const fn = await getInvoke();
  return fn('delete_experience', { id }) as Promise<boolean>;
}

export async function addEdge(data: {
  source_id: string; source_type: string; target_id: string;
  target_type: string; relation: string; weight: number;
}): Promise<Edge> {
  const fn = await getInvoke();
  return fn('add_edge', data) as Promise<Edge>;
}

export async function removeEdge(edge_id: string): Promise<boolean> {
  const fn = await getInvoke();
  return fn('remove_edge', { edge_id }) as Promise<boolean>;
}

export async function listEdgesForEntity(entity_id: string, entity_type: string): Promise<Edge[]> {
  const fn = await getInvoke();
  return fn('list_edges_for_entity', { entity_id, entity_type }) as Promise<Edge[]>;
}

export async function listAllEdges(): Promise<Edge[]> {
  const fn = await getInvoke();
  return fn('list_all_edges') as Promise<Edge[]>;
}

export async function listCompanies() {
  const fn = await getInvoke();
  return fn('list_companies');
}

export async function getGraphData(): Promise<GraphData> {
  const fn = await getInvoke();
  return fn('get_graph_data') as Promise<GraphData>;
}

// ── Enhanced Search ───────────────────────────────────────────────────

export interface EnhancedSearchResult {
  entity_type: string;
  id: string;
  name: string;
  relevance_score: number;
  matched_terms: string[];
  snippet: string;
}

export async function searchKnowledgeEnhanced(query: string): Promise<EnhancedSearchResult[]> {
  const fn = await getInvoke();
  return fn('search_knowledge_enhanced', { query }) as Promise<EnhancedSearchResult[]>;
}

// ── Meeting Analysis ──────────────────────────────────────────────────

export interface MeetingSuggestion {
  suggestion_type: 'skill' | 'star_story' | 'improvement' | 'strong_answer';
  title: string;
  description: string;
  confidence: number;
  data: Record<string, unknown>;
}

export interface MeetingAnalysis {
  suggestions: MeetingSuggestion[];
  summary: string;
  skills_found: string[];
}

export async function analyzeMeeting(transcript: string): Promise<MeetingAnalysis> {
  const fn = await getInvoke();
  return fn('analyze_meeting', { transcript }) as Promise<MeetingAnalysis>;
}

// ── Practice Mode ─────────────────────────────────────────────────────

export interface PracticeQuestion {
  question_type: string;
  question: string;
  tips: string;
  expected_aspects: string[];
}

export interface AnswerFeedback {
  structure_score: number;
  specificity_score: number;
  relevance_score: number;
  overall_score: number;
  feedback: string;
  improvements: string[];
  strong_points: string[];
}

export async function generatePracticeQuestions(mode: string, companyName?: string): Promise<PracticeQuestion[]> {
  const fn = await getInvoke();
  return fn('generate_practice_questions', { mode, companyName }) as Promise<PracticeQuestion[]>;
}

export async function analyzePracticeAnswer(question: string, answer: string, mode: string): Promise<AnswerFeedback> {
  const fn = await getInvoke();
  return fn('analyze_practice_answer', { question, answer, mode }) as Promise<AnswerFeedback>;
}

export type { KnowledgeGraphStats, SearchResult };
