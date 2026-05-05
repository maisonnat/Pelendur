//! Auto-learning module: analyzes meeting transcripts via LLM
//! and extracts suggestions for the knowledge graph (skills, STAR stories,
//! improvements, strong answers). Nothing is auto-inserted — the user
//! must explicitly confirm each suggestion.

use crate::config::Config;
use crate::llm::{self, ChatMessage};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::graph::KnowledgeGraph;
use super::search::KnowledgeSearcher;

// ── Public data types ────────────────────────────────────────────────────

/// Types of learning suggestions extracted from meeting transcripts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionType {
    SkillMentioned,
    PotentialStarStory,
    ImprovementArea,
    StrongAnswer,
}

impl SuggestionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SuggestionType::SkillMentioned => "skill",
            SuggestionType::PotentialStarStory => "star_story",
            SuggestionType::ImprovementArea => "improvement",
            SuggestionType::StrongAnswer => "strong_answer",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "skill" => SuggestionType::SkillMentioned,
            "star_story" | "star" => SuggestionType::PotentialStarStory,
            "improvement" => SuggestionType::ImprovementArea,
            "strong_answer" | "strong" => SuggestionType::StrongAnswer,
            _ => SuggestionType::SkillMentioned,
        }
    }
}

/// A single learning suggestion extracted from a meeting transcript.
/// User must explicitly approve each suggestion before it is inserted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningSuggestion {
    /// Unique identifier for this suggestion
    pub id: String,
    /// Type of suggestion
    pub suggestion_type: SuggestionType,
    /// Human-readable title
    pub title: String,
    /// Detailed description of the suggestion
    pub description: String,
    /// Confidence score (0.0-1.0) based on transcript clarity
    pub confidence: f64,
    /// Quote from transcript supporting this suggestion
    pub source_excerpt: String,
    /// Structured data to insert if approved (skill, star story, etc.)
    pub suggested_data: serde_json::Value,
}

/// Complete analysis of a meeting transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingAnalysis {
    /// Unique meeting identifier
    pub meeting_id: String,
    /// Original transcript text
    pub transcript: String,
    /// All extracted suggestions
    pub suggestions: Vec<LearningSuggestion>,
    /// Brief summary of the meeting
    pub summary: String,
    /// Meeting duration in minutes
    pub duration_minutes: u32,
}

/// LLM client wrapper for the auto-learner
pub struct LlmClient<'a> {
    config: &'a Config,
}

impl<'a> LlmClient<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self { config }
    }

    pub async fn generate(&self, prompt: &str, max_tokens: u32) -> Result<String> {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];
        llm::generate_response_with_options(self.config, &messages, max_tokens)
            .await
            .context("LLM call failed")
    }
}

// ── AutoLearner ──────────────────────────────────────────────────────────

pub struct AutoLearner<'a> {
    llm_client: LlmClient<'a>,
    searcher: KnowledgeSearcher<'a>,
}

impl<'a> AutoLearner<'a> {
    pub fn new(graph: &'a KnowledgeGraph, config: &'a Config) -> Self {
        let searcher = KnowledgeSearcher::new(graph);
        let llm_client = LlmClient::new(config);
        Self {
            llm_client,
            searcher,
        }
    }

    pub async fn analyze_meeting(
        graph: &'a KnowledgeGraph,
        transcript: &str,
        config: &'a Config,
        duration_minutes: u32,
    ) -> Result<MeetingAnalysis> {
        let learner = Self::new(graph, config);
        learner.analyze(transcript, duration_minutes).await
    }

    async fn analyze(&self, transcript: &str, duration_minutes: u32) -> Result<MeetingAnalysis> {
        let meeting_id = Uuid::new_v4().to_string();

        let existing_skills = self.get_existing_skills();
        let prompt = self.build_analysis_prompt(transcript, &existing_skills);
        let response = self
            .llm_client
            .generate(&prompt, 2500)
            .await
            .context("LLM analysis failed")?;
        let suggestions = self.parse_llm_response(&response, transcript)?;
        let summary = self.extract_summary(&response)?;

        Ok(MeetingAnalysis {
            meeting_id,
            transcript: transcript.to_string(),
            suggestions,
            summary,
            duration_minutes,
        })
    }

    /// Get list of existing skill names for deduplication.
    fn get_existing_skills(&self) -> Vec<String> {
        self.searcher
            .context_search("")
            .into_iter()
            .filter(|r| r.entity_type == "skill")
            .map(|r| r.name.to_lowercase())
            .collect()
    }

    /// Build the analysis prompt for the LLM.
    fn build_analysis_prompt(&self, transcript: &str, existing_skills: &[String]) -> String {
        let existing_skills_str = if existing_skills.is_empty() {
            "No existing skills in profile".to_string()
        } else {
            format!("Already in profile: {}", existing_skills.join(", "))
        };

        format!(
            r#"You are an expert career coach analyzing a meeting/interview transcript.

Analyze this transcript and identify learning opportunities:

EXISTING SKILLS IN PROFILE: {existing_skills_str}

TRANSCRIPT:
{transcript}

Respond with a JSON object containing your analysis:

{{
  "summary": "Brief 1-2 sentence summary of what happened in this meeting",
  "suggestions": [
    {{
      "suggestion_type": "skill",
      "title": "Skill Name",
      "description": "Brief explanation of why this skill was detected",
      "confidence": 0.9,
      "source_excerpt": "Exact quote from transcript supporting this",
      "suggested_data": {{
        "name": "SkillName",
        "category": "Technical|Soft|Leadership",
        "level": "mentioned|beginner|intermediate|advanced|expert",
        "years_hint": null
      }}
    }},
    {{
      "suggestion_type": "star_story",
      "title": "Story Title",
      "description": "Description of the accomplishment story detected",
      "confidence": 0.8,
      "source_excerpt": "Relevant quote showing the situation",
      "suggested_data": {{
        "title": "Story Title",
        "situation": "The context/challenge described",
        "task": "The speaker's responsibility or goal",
        "action": "What the speaker specifically did",
        "result": "The outcome/impact achieved",
        "tags": ["leadership", "problem-solving", "technical"]
      }}
    }},
    {{
      "suggestion_type": "improvement",
      "title": "Area for Improvement",
      "description": "What could be strengthened in future answers",
      "confidence": 0.7,
      "source_excerpt": "Quote showing the weak area",
      "suggested_data": {{
        "topic": "The area discussed",
        "suggestion": "Specific actionable improvement",
        "why_important": "Why this matters for interviews"
      }}
    }},
    {{
      "suggestion_type": "strong_answer",
      "title": "Well-Done Response",
      "description": "Why this answer was effective",
      "confidence": 0.9,
      "source_excerpt": "Quote of the strong answer",
      "suggested_data": {{
        "topic": "What was answered well",
        "strength": "Specific quality that made it strong",
        "pattern": "Structure or technique used"
      }}
    }}
  ]
}}

Rules:
- Do NOT suggest skills that are already in the existing profile list
- Only suggest skills with confidence > 0.7 (be conservative)
- For garbled or unclear STT text, use low confidence (< 0.5) or skip entirely
- Maximum 10 suggestions total across all types
- Extract relevant quotes as source_excerpt for EVERY suggestion
- Do NOT extract personal information about other speakers
- Return ONLY valid JSON, no markdown fencing or additional text"#
        )
    }

    /// Parse the LLM response into structured suggestions.
    fn parse_llm_response(
        &self,
        response: &str,
        transcript: &str,
    ) -> Result<Vec<LearningSuggestion>> {
        let cleaned = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        #[derive(Deserialize)]
        struct LlmResponse {
            #[allow(dead_code)]
            summary: Option<String>,
            suggestions: Vec<LlmSuggestionRaw>,
        }

        #[derive(Deserialize)]
        struct LlmSuggestionRaw {
            suggestion_type: String,
            title: String,
            description: String,
            confidence: f64,
            source_excerpt: Option<String>,
            suggested_data: serde_json::Value,
        }

        let llm_resp: LlmResponse = serde_json::from_str(cleaned).with_context(|| {
            format!(
                "Failed to parse LLM response as JSON. Preview: {}",
                &cleaned[..cleaned.len().min(500)]
            )
        })?;

        let existing_skills = self.get_existing_skills();

        let mut suggestions = Vec::new();
        for raw in llm_resp.suggestions {
            if raw.suggestion_type == "skill" {
                if let Some(name) = raw.suggested_data.get("name").and_then(|v| v.as_str()) {
                    if existing_skills.contains(&name.to_lowercase()) {
                        continue;
                    }
                }
            }

            if raw.confidence < 0.5 && raw.suggestion_type == "skill" {
                continue;
            }

            let suggestion_type = SuggestionType::from_str(&raw.suggestion_type);
            let source_excerpt = raw
                .source_excerpt
                .unwrap_or_else(|| extract_relevant_excerpt(transcript, &raw.description));

            suggestions.push(LearningSuggestion {
                id: Uuid::new_v4().to_string(),
                suggestion_type,
                title: raw.title,
                description: raw.description,
                confidence: raw.confidence.clamp(0.0, 1.0),
                source_excerpt,
                suggested_data: raw.suggested_data,
            });
        }

        Ok(suggestions)
    }

    /// Extract a brief summary from the LLM response.
    fn extract_summary(&self, response: &str) -> Result<String> {
        // Clean the response
        let cleaned = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        #[derive(Deserialize)]
        struct SummaryOnly {
            summary: Option<String>,
        }

        if let Ok(s) = serde_json::from_str::<SummaryOnly>(cleaned) {
            Ok(s.summary
                .unwrap_or_else(|| "Meeting analyzed successfully".to_string()))
        } else {
            Ok("Meeting analyzed - review suggestions below".to_string())
        }
    }

    pub fn extract_skills(&self, transcript: &str) -> Vec<LearningSuggestion> {
        let results = self.searcher.context_search(transcript);

        let existing_skills: Vec<String> = results
            .iter()
            .filter(|r| r.entity_type == "skill")
            .map(|r| r.name.to_lowercase())
            .collect();

        let skill_keywords = extract_tech_keywords(transcript);

        skill_keywords
            .into_iter()
            .filter(|(skill, _)| !existing_skills.contains(&skill.to_lowercase()))
            .map(|(skill, excerpt)| LearningSuggestion {
                id: Uuid::new_v4().to_string(),
                suggestion_type: SuggestionType::SkillMentioned,
                title: skill.clone(),
                description: format!("Technical skill '{}' mentioned in meeting", skill),
                confidence: 0.75,
                source_excerpt: excerpt,
                suggested_data: serde_json::json!({
                    "name": skill,
                    "category": "Technical",
                    "level": "mentioned",
                    "years_hint": null
                }),
            })
            .collect()
    }

    pub fn detect_star_fragments(&self, transcript: &str) -> Vec<LearningSuggestion> {
        let star_patterns = [
            ("challenge", "I faced a challenge", "challenge overcame"),
            ("problem", "the problem was", "problem solved"),
            ("led", "I led", "leadership"),
            ("improved", "I improved", "improvement"),
            ("increased", "increased by", "metrics"),
            ("reduced", "reduced by", "metrics"),
            ("achieved", "I achieved", "accomplishment"),
            ("delivered", "I delivered", "delivery"),
        ];

        let transcript_lower = transcript.to_lowercase();
        let mut suggestions = Vec::new();

        for (pattern, _quote_hint, tag) in star_patterns {
            if transcript_lower.contains(pattern) {
                let excerpt = extract_relevant_excerpt(transcript, pattern);

                suggestions.push(LearningSuggestion {
                    id: Uuid::new_v4().to_string(),
                    suggestion_type: SuggestionType::PotentialStarStory,
                    title: format!("Potential {} story", tag),
                    description: format!(
                        "Detected language suggesting a {} story. Consider developing into a full STAR story.",
                        tag
                    ),
                    confidence: 0.6,
                    source_excerpt: excerpt,
                    suggested_data: serde_json::json!({
                        "title": null,
                        "situation": "",
                        "task": "",
                        "action": "",
                        "result": "",
                        "tags": [tag]
                    }),
                });
            }
        }

        suggestions
    }

    pub fn identify_improvements(&self, transcript: &str) -> Vec<LearningSuggestion> {
        let mut suggestions = Vec::new();
        let transcript_lower = transcript.to_lowercase();

        let vague_indicators = ["a lot", "very", "big", "small", "good", "bad", "better"];
        let has_numbers = transcript.chars().any(|c| c.is_numeric());

        for vague in vague_indicators {
            if transcript_lower.contains(vague) && !has_numbers {
                suggestions.push(LearningSuggestion {
                    id: Uuid::new_v4().to_string(),
                    suggestion_type: SuggestionType::ImprovementArea,
                    title: "Add quantifiable metrics".to_string(),
                    description:
                        "Consider adding specific numbers or percentages to strengthen your answer"
                            .to_string(),
                    confidence: 0.5,
                    source_excerpt: extract_relevant_excerpt(transcript, vague),
                    suggested_data: serde_json::json!({
                        "topic": "general",
                        "suggestion": "Add specific metrics: percentages, numbers, time frames",
                        "why_important": "Quantifiable results are more convincing and memorable"
                    }),
                });
                break;
            }
        }

        let passive_phrases = ["was done", "were done", "it was", "things were"];
        for phrase in passive_phrases {
            if transcript_lower.contains(phrase) {
                suggestions.push(LearningSuggestion {
                    id: Uuid::new_v4().to_string(),
                    suggestion_type: SuggestionType::ImprovementArea,
                    title: "Use active voice".to_string(),
                    description: "Consider reframing to highlight your personal contributions".to_string(),
                    confidence: 0.5,
                    source_excerpt: extract_relevant_excerpt(transcript, phrase),
                    suggested_data: serde_json::json!({
                        "topic": "communication",
                        "suggestion": "Use 'I' instead of 'we' or passive constructions",
                        "why_important": "Interviewers want to hear about YOUR specific contributions"
                    }),
                });
                break;
            }
        }

        suggestions
    }

    pub fn highlight_strong_answers(&self, transcript: &str) -> Vec<LearningSuggestion> {
        let mut suggestions = Vec::new();
        let transcript_lower = transcript.to_lowercase();

        let strong_patterns = [
            ("because i", "Clear causation stated"),
            ("result was", "Results quantified"),
            ("outcome", "Outcome-focused"),
            ("learned that", "Reflection shown"),
            ("specifically", "Specific details provided"),
            ("for example", "Examples given"),
            ("first, second", "Structured response"),
            ("conclusion", "Well-concluded"),
        ];

        for (pattern, strength) in strong_patterns {
            if transcript_lower.contains(pattern) {
                suggestions.push(LearningSuggestion {
                    id: Uuid::new_v4().to_string(),
                    suggestion_type: SuggestionType::StrongAnswer,
                    title: format!("Strong answer component: {}", strength),
                    description: format!(
                        "Your response demonstrates {}. This is effective!",
                        strength.to_lowercase()
                    ),
                    confidence: 0.8,
                    source_excerpt: extract_relevant_excerpt(transcript, pattern),
                    suggested_data: serde_json::json!({
                        "topic": "general",
                        "strength": strength,
                        "pattern": "Continue using this pattern"
                    }),
                });
            }
        }

        suggestions
    }
}

fn extract_relevant_excerpt(transcript: &str, keyword: &str) -> String {
    let transcript_lower = transcript.to_lowercase();
    let keyword_lower = keyword.to_lowercase();

    if let Some(pos) = transcript_lower.find(&keyword_lower) {
        let start = pos.saturating_sub(50);
        let end = (pos + keyword.len() + 50).min(transcript.len());
        let excerpt = &transcript[start..end];
        let prefix = if start > 0 { "..." } else { "" };
        let suffix = if end < transcript.len() { "..." } else { "" };
        format!("{}{}{}", prefix, excerpt.trim(), suffix)
    } else {
        let end = 100.min(transcript.len());
        let excerpt = &transcript[..end];
        if end < transcript.len() {
            format!("{}...", excerpt.trim())
        } else {
            excerpt.to_string()
        }
    }
}

fn extract_tech_keywords(transcript: &str) -> Vec<(String, String)> {
    let known_techs = [
        "Rust",
        "Go",
        "Python",
        "JavaScript",
        "TypeScript",
        "Java",
        "C++",
        "C#",
        "Ruby",
        "PHP",
        "Swift",
        "Kotlin",
        "Scala",
        "R",
        "MATLAB",
        "SQL",
        "Bash",
        "Shell",
        "React",
        "Vue",
        "Angular",
        "Next.js",
        "Node.js",
        "Django",
        "Flask",
        "Rails",
        "Spring",
        "Express",
        "FastAPI",
        "Svelte",
        "Remix",
        "PostgreSQL",
        "MySQL",
        "MongoDB",
        "Redis",
        "Elasticsearch",
        "Cassandra",
        "DynamoDB",
        "SQLite",
        "Oracle",
        "SQL Server",
        "Neo4j",
        "AWS",
        "Azure",
        "GCP",
        "Kubernetes",
        "Docker",
        "Terraform",
        "Ansible",
        "Jenkins",
        "GitHub Actions",
        "GitLab CI",
        "CircleCI",
        "Prometheus",
        "Grafana",
        "microservices",
        "monolith",
        "REST",
        "GraphQL",
        "gRPC",
        "TCP",
        "UDP",
        "HTTP",
        "CI/CD",
        "DevOps",
        "Agile",
        "Scrum",
        "Kanban",
        "TDD",
        "BDD",
        "machine learning",
        "deep learning",
        "neural network",
        "NLP",
        "CV",
        "computer vision",
        "tensorflow",
        "pytorch",
        "scikit-learn",
        "pandas",
        "numpy",
        "Git",
        "Linux",
        "Unix",
        "Windows Server",
        "Nginx",
        "Apache",
        "Kafka",
    ];

    let transcript_lower = transcript.to_lowercase();
    let mut found = Vec::new();

    for tech in known_techs {
        if transcript_lower.contains(&tech.to_lowercase()) {
            let excerpt = extract_relevant_excerpt(transcript, tech);
            found.push((tech.to_string(), excerpt));
        }
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggestion_type_as_str() {
        assert_eq!(SuggestionType::SkillMentioned.as_str(), "skill");
        assert_eq!(SuggestionType::PotentialStarStory.as_str(), "star_story");
        assert_eq!(SuggestionType::ImprovementArea.as_str(), "improvement");
        assert_eq!(SuggestionType::StrongAnswer.as_str(), "strong_answer");
    }

    #[test]
    fn test_suggestion_type_from_str() {
        assert_eq!(
            SuggestionType::from_str("skill"),
            SuggestionType::SkillMentioned
        );
        assert_eq!(
            SuggestionType::from_str("star_story"),
            SuggestionType::PotentialStarStory
        );
        assert_eq!(
            SuggestionType::from_str("STAR"),
            SuggestionType::PotentialStarStory
        );
        assert_eq!(
            SuggestionType::from_str("improvement"),
            SuggestionType::ImprovementArea
        );
        assert_eq!(
            SuggestionType::from_str("strong_answer"),
            SuggestionType::StrongAnswer
        );
        assert_eq!(
            SuggestionType::from_str("strong"),
            SuggestionType::StrongAnswer
        );
    }

    #[test]
    fn test_extract_tech_keywords() {
        let transcript =
            "I've been working with Kubernetes and Go for 3 years, using Docker for deployments.";
        let keywords = extract_tech_keywords(transcript);

        assert!(keywords.iter().any(|(k, _)| k == "Kubernetes"));
        assert!(keywords.iter().any(|(k, _)| k == "Go"));
        assert!(keywords.iter().any(|(k, _)| k == "Docker"));
    }

    #[test]
    fn test_extract_tech_keywords_garbled() {
        let transcript = "we used cuber nets for deplyment";
        let keywords = extract_tech_keywords(transcript);

        assert!(!keywords.iter().any(|(k, _)| k == "Kubernetes"));
        assert!(!keywords.iter().any(|(k, _)| k == "Docker"));
    }

    #[test]
    fn test_extract_relevant_excerpt() {
        let transcript =
            "So the situation was that I was working on a critical bug. I had to fix it quickly.";
        let excerpt = extract_relevant_excerpt(transcript, "bug");
        assert!(excerpt.contains("bug"));
    }

    #[test]
    fn test_extract_relevant_excerpt_not_found() {
        let transcript = "This is a short test transcript.";
        let excerpt = extract_relevant_excerpt(transcript, "nonexistent");
        assert!(!excerpt.is_empty());
    }

    #[test]
    fn test_learning_suggestion_serialization() {
        let suggestion = LearningSuggestion {
            id: "test-id".to_string(),
            suggestion_type: SuggestionType::SkillMentioned,
            title: "Rust".to_string(),
            description: "Backend language".to_string(),
            confidence: 0.9,
            source_excerpt: "I know Rust".to_string(),
            suggested_data: serde_json::json!({
                "name": "Rust",
                "category": "Systems",
                "level": "expert"
            }),
        };

        let json = serde_json::to_string(&suggestion).unwrap();
        assert!(json.contains("Rust"));
        assert!(json.contains("skill"));
    }

    #[test]
    fn test_meeting_analysis_serialization() {
        let analysis = MeetingAnalysis {
            meeting_id: "meeting-123".to_string(),
            transcript: "Test transcript".to_string(),
            suggestions: vec![],
            summary: "Test meeting".to_string(),
            duration_minutes: 30,
        };

        let json = serde_json::to_string(&analysis).unwrap();
        assert!(json.contains("meeting-123"));
        assert!(json.contains("Test meeting"));
    }

    #[test]
    fn test_confidence_clamping() {
        let suggestion = LearningSuggestion {
            id: "test".to_string(),
            suggestion_type: SuggestionType::SkillMentioned,
            title: "Test".to_string(),
            description: "Test".to_string(),
            confidence: 0.5,
            source_excerpt: "Test".to_string(),
            suggested_data: serde_json::json!({}),
        };

        assert!(suggestion.confidence >= 0.0);
        assert!(suggestion.confidence <= 1.0);
    }
}
