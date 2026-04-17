export interface StoryFormData {
  id?: string;
  title: string;
  situation: string;
  task: string;
  action: string;
  result: string;
  tags: string[];
  difficulty: string;
  stakes: string;
}

export type ViewMode = "list" | "editor" | "preview" | "practice";

export const EMPTY_STORY: StoryFormData = {
  title: "",
  situation: "",
  task: "",
  action: "",
  result: "",
  tags: [],
  difficulty: "",
  stakes: "",
};

export const SUGGESTED_TAGS = [
  "leadership",
  "communication",
  "teamwork",
  "problem-solving",
  "conflict-resolution",
  "performance",
  "architecture",
  "migration",
  "crisis",
  "ownership",
  "mentoring",
  "innovation",
  "backend",
  "frontend",
  "devops",
  "database",
  "optimization",
  "caching",
  "microservices",
  "team-management",
  "stakeholder-management",
  "presentation",
  "business-acumen",
  "mediation",
  "team-dynamics",
  "strangler-fig",
  "negotiation",
  "growth",
  "impact",
  "technical-debt",
] as const;

export const COMPETENCY_TEMPLATES: Record<string, Partial<StoryFormData>> = {
  leadership: {
    title: "",
    situation: "When the team faced [challenge] during [context]...",
    task: "I needed to [specific responsibility] while [constraint]...",
    action: "I [specific action 1], then [specific action 2], and [specific action 3]...",
    result: "The outcome was [quantifiable result], which led to [impact]...",
    tags: ["leadership", "ownership"],
    difficulty: "medium",
    stakes: "high",
  },
  conflict: {
    title: "",
    situation: "There was a disagreement between [parties] about [topic]...",
    task: "I needed to mediate and find a resolution that [goal]...",
    action: "I [mediation step 1], identified [root cause], and proposed [solution]...",
    result: "We reached [resolution], which improved [metric] and [relationship outcome]...",
    tags: ["conflict-resolution", "communication"],
    difficulty: "medium",
    stakes: "medium",
  },
  growth: {
    title: "",
    situation: "I recognized a gap in my [skill area] when [trigger event]...",
    task: "I set out to [learning goal] within [timeframe]...",
    action: "I [learning method 1], applied it by [project], and sought feedback from [mentor/source]...",
    result: "I achieved [measurable improvement], which enabled [new capability]...",
    tags: ["growth", "learning"],
    difficulty: "easy",
    stakes: "low",
  },
  impact: {
    title: "",
    situation: "The team/organization was experiencing [problem] that affected [stakeholders]...",
    task: "I was tasked with [specific goal] to address [metric/outcome]...",
    action: "I [technical approach], collaborated with [team/stakeholders], and delivered [artifact]...",
    result: "This resulted in [X% improvement] in [metric], saving [cost/time] and [broader impact]...",
    tags: ["impact", "backend"],
    difficulty: "hard",
    stakes: "high",
  },
};
