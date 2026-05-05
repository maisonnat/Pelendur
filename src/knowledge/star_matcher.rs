//! STAR Matching Engine — semantic matching between interview questions
//! and STAR stories from the user's profile.
//!
//! Combines hash-based embeddings (fast, local, no API calls), tag/FTS
//! keyword search, and knowledge graph edge enrichment to find the most
//! relevant stories for any interview context.
//!
//! Scoring formula:
//!   composite = 0.50 * embedding_sim + 0.25 * keyword_score + 0.15 * edge_boost + 0.10 * balance_factor
//!
//! Where:
//!   - embedding_sim  → cosine similarity between query and story hash vectors
//!   - keyword_score  → FTS5 / tag / fuzzy match score from EnhancedSearch
//!   - edge_boost     → sum(edge.weight) for edges linking story ↔ skill/project/company
//!   - balance_factor → preference for stories with lower usage_count

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::embeddings::HashEmbeddingService;
use super::graph::{EntityType, KnowledgeGraph, StarStoryEntity};
use super::search::{EnhancedSearch, KnowledgeSearcher, SearchOptions};

// ── Public types ─────────────────────────────────────────────────────────

/// A matched STAR story with complete metadata and context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarMatch {
    /// The full STAR story entity from the knowledge graph
    pub story: StarStoryEntity,
    /// Composite relevance score (0.0 – 1.0)
    pub relevance_score: f64,
    /// Semantic embedding similarity (0.0 – 1.0)
    pub embedding_similarity: f64,
    /// Keyword / FTS / tag match score (0.0 – 1.0)
    pub keyword_score: f64,
    /// Boost from graph edges (linked skills, projects, companies)
    pub edge_boost: f64,
    /// Skills associated with this story via graph edges
    pub linked_skills: Vec<LinkedEntity>,
    /// Projects associated with this story
    pub linked_projects: Vec<LinkedEntity>,
    /// Companies associated with this story
    pub linked_companies: Vec<LinkedEntity>,
    /// Key terms that matched between query and story
    pub matched_terms: Vec<String>,
}

/// A linked entity (skill, project, company) with relevance info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedEntity {
    pub id: String,
    pub name: String,
    pub relation: String,
    pub weight: f64,
}

/// Configuration for STAR matching
#[derive(Debug, Clone)]
pub struct StarMatchOptions {
    /// Maximum number of stories to return
    pub max_results: usize,
    /// Minimum composite score threshold
    pub min_score: f64,
    /// Boost stories with fresh embeddings (recompute if missing)
    pub auto_embed: bool,
    /// Prefer less-used stories for balanced practice
    pub balance_usage: bool,
    /// Weight for embedding similarity in composite score
    pub embedding_weight: f64,
    /// Weight for keyword search score
    pub keyword_weight: f64,
    /// Weight for graph edge boost
    pub edge_weight: f64,
    /// Weight for usage balance factor
    pub balance_weight: f64,
}

impl Default for StarMatchOptions {
    fn default() -> Self {
        Self {
            max_results: 5,
            min_score: 0.15,
            auto_embed: true,
            balance_usage: true,
            embedding_weight: 0.50,
            keyword_weight: 0.25,
            edge_weight: 0.15,
            balance_weight: 0.10,
        }
    }
}

// ── StarMatcher ──────────────────────────────────────────────────────────

/// Main STAR matching engine.
///
/// Usage:
/// ```no_run
/// let matcher = StarMatcher::new(&graph);
/// let matches = matcher.match_stories("Tell me about a time you resolved a conflict", None)?;
/// ```
pub struct StarMatcher<'a> {
    graph: &'a KnowledgeGraph,
    embedding_service: HashEmbeddingService,
    searcher: KnowledgeSearcher<'a>,
    enhanced: EnhancedSearch<'a>,
}

impl<'a> StarMatcher<'a> {
    /// Create a new StarMatcher backed by the given knowledge graph.
    pub fn new(graph: &'a KnowledgeGraph) -> Self {
        let searcher = KnowledgeSearcher::new(graph);
        let enhanced = EnhancedSearch::new(graph);
        Self {
            graph,
            embedding_service: HashEmbeddingService::default(),
            searcher,
            enhanced,
        }
    }

    // ── Public API ────────────────────────────────────────────────────

    /// Find the most relevant STAR stories for a given interview context.
    ///
    /// `context` is an interview question, transcribed snippet, or any
    /// natural language text. Returns ranked `StarMatch` results.
    pub fn match_stories(
        &self,
        context: &str,
        options: Option<StarMatchOptions>,
    ) -> Result<Vec<StarMatch>> {
        if context.trim().is_empty() || context.len() < 5 {
            return Ok(Vec::new());
        }

        let opts = options.unwrap_or_default();
        let context = context.trim();

        // 1. Ensure embeddings exist if auto_embed is enabled
        if opts.auto_embed {
            self.ensure_story_embeddings()?;
        }

        // 2. Fetch all stories from the graph
        let stories = self.graph.list_star_stories()?;
        if stories.is_empty() {
            return Ok(Vec::new());
        }

        // 3. Pre-compute query embedding once
        let query_vector = self.embedding_service.embed(context);

        // 4. Compute keyword search results (FTS + fuzzy + tags)
        let keyword_results = self.search_keywords(context, &opts)?;

        // 5. Compute match for each story
        let mut matches: Vec<StarMatch> = Vec::new();

        for story in &stories {
            let story_text = self.build_story_embedding_text(story);
            let story_vector = self.embedding_service.embed(&story_text);
            let embedding_sim =
                self.embedding_service
                    .cosine_similarity(&query_vector, &story_vector) as f64;

            // Keyword score: use existing search results or default to embedding_sim * 0.5
            let keyword_score = keyword_results
                .get(&story.id)
                .copied()
                .unwrap_or_else(|| embedding_sim * 0.5)
                .clamp(0.0, 1.0);

            // Edge boost: linked skills, projects, companies
            let (edge_boost, linked_skills, linked_projects, linked_companies) =
                self.compute_edge_context(story, context);

            // Usage balance factor: prefer less-used stories
            let balance_factor = if opts.balance_usage && story.usage_count > 0 {
                // Normalize: usage_count of 0 → 1.0, usage_count of 50+ → ~0.3
                (1.0 / (1.0 + (story.usage_count as f64 * 0.05))).clamp(0.2, 1.0)
            } else {
                1.0
            };

            // Composite score
            let composite = opts.embedding_weight * embedding_sim
                + opts.keyword_weight * keyword_score
                + opts.edge_weight * edge_boost.min(1.0)
                + opts.balance_weight * balance_factor;

            if composite >= opts.min_score {
                let matched_terms = self.extract_matched_terms(context, story);
                matches.push(StarMatch {
                    story: story.clone(),
                    relevance_score: composite,
                    embedding_similarity: embedding_sim,
                    keyword_score,
                    edge_boost: edge_boost.min(1.0),
                    linked_skills,
                    linked_projects,
                    linked_companies,
                    matched_terms,
                });
            }
        }

        // 6. Sort by composite score descending
        matches.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        matches.truncate(opts.max_results);
        Ok(matches)
    }

    /// Quick search that only returns stories matching specific tags/skills
    pub fn match_by_tags(&self, tags: &[String]) -> Result<Vec<StarMatch>> {
        let results = self.searcher.search_stories_by_tags(tags)?;
        let story_ids: Vec<String> = results.iter().map(|r| r.entity_id.clone()).collect();

        let all_stories = self.graph.list_star_stories()?;
        let story_map: HashMap<String, StarStoryEntity> =
            all_stories.into_iter().map(|s| (s.id.clone(), s)).collect();

        let mut matches = Vec::new();
        for id in &story_ids {
            if let Some(story) = story_map.get(id) {
                let (_, linked_skills, linked_projects, linked_companies) =
                    self.compute_edge_context(story, "");
                matches.push(StarMatch {
                    story: story.clone(),
                    relevance_score: 0.8,
                    embedding_similarity: 0.0,
                    keyword_score: 0.8,
                    edge_boost: 0.0,
                    linked_skills,
                    linked_projects,
                    linked_companies,
                    matched_terms: tags.to_vec(),
                });
            }
        }

        Ok(matches)
    }

    /// Increment usage_count for a story after it's been suggested/shown
    pub fn record_usage(&self, story_id: &str) -> Result<()> {
        if let Some(mut story) = self.graph.get_star_story(story_id)? {
            story.usage_count += 1;
            self.graph.update_star_story(&story)?;
        }
        Ok(())
    }

    // ── Internal helpers ──────────────────────────────────────────────

    /// Ensure all STAR stories have embeddings in the persistent store
    fn ensure_story_embeddings(&self) -> Result<()> {
        let store = super::embeddings::PersistentVectorStore::new(self.graph.conn());
        store
            .init_tables()
            .context("Failed to init embedding tables")?;

        // Check if stories need embedding by trying to get a count
        let total_stories = self.graph.list_star_stories()?.len();
        let stored_count = store.count().unwrap_or(0);

        if stored_count < total_stories {
            store
                .generate_story_embeddings(self.graph.conn())
                .context("Failed to generate story embeddings")?;
        }
        Ok(())
    }

    /// Build the full text for embedding a STAR story (combines all fields + tags)
    fn build_story_embedding_text(&self, story: &StarStoryEntity) -> String {
        let tags_text = parse_tags_json(&story.tags).join(" ");
        format!(
            "{} {} {} {} {} {}",
            story.title.as_deref().unwrap_or_default(),
            story.situation,
            story.task,
            story.action,
            story.result,
            tags_text
        )
    }

    /// Search for context keywords using EnhancedSearch (FTS5 + fuzzy + tags)
    fn search_keywords(
        &self,
        context: &str,
        _opts: &StarMatchOptions,
    ) -> Result<HashMap<String, f64>> {
        let mut result_map = HashMap::new();

        // Extract meaningful words (3+ chars)
        let words: Vec<String> = context
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .map(|w| {
                w.to_lowercase()
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string()
            })
            .filter(|w| !w.is_empty())
            .collect();

        let search_opts = SearchOptions {
            max_results: 50,
            fuzzy_threshold: 0.45,
            ..Default::default()
        };

        // Search with full context for FTS
        if let Ok(results) = self.enhanced.search(context, &search_opts) {
            for r in results {
                if r.entity_type == "star_story" {
                    let entry = result_map.entry(r.id.clone()).or_insert(0.0_f64);
                    *entry = r.relevance_score.max(*entry);
                }
            }
        }

        // Also search individual key words for broader coverage
        for word in words.iter().take(10) {
            if let Ok(results) = self.enhanced.search(word, &search_opts) {
                for r in results {
                    if r.entity_type == "star_story" {
                        let entry = result_map.entry(r.id.clone()).or_insert(0.0_f64);
                        *entry = r.relevance_score.max(*entry);
                    }
                }
            }
        }

        Ok(result_map)
    }

    /// Compute the edge boost and collect linked entities for a story
    fn compute_edge_context(
        &self,
        story: &StarStoryEntity,
        query: &str,
    ) -> (f64, Vec<LinkedEntity>, Vec<LinkedEntity>, Vec<LinkedEntity>) {
        let edges = match self
            .graph
            .get_edges_for_entity(&story.id, EntityType::StarStory)
        {
            Ok(e) => e,
            Err(_) => return (0.0, Vec::new(), Vec::new(), Vec::new()),
        };

        let mut total_boost = 0.0_f64;
        let mut skills = Vec::new();
        let mut projects = Vec::new();
        let mut companies = Vec::new();
        let query_lower = query.to_lowercase();

        for edge in &edges {
            let connected_id = if edge.source_id == story.id {
                &edge.target_id
            } else {
                &edge.source_id
            };
            let connected_type = if edge.source_id == story.id {
                &edge.target_type
            } else {
                &edge.source_type
            };

            total_boost += edge.weight;

            match connected_type.as_str() {
                "skill" => {
                    if let Ok(Some(skill)) = self.graph.get_skill(connected_id) {
                        skills.push(LinkedEntity {
                            id: skill.id,
                            name: skill.name,
                            relation: edge.relation.clone(),
                            weight: edge.weight,
                        });
                    }
                }
                "project" => {
                    if let Ok(Some(proj)) = self.graph.get_project(connected_id) {
                        // Check if project keywords match query context
                        let kw_score = proj
                            .keywords
                            .as_deref()
                            .map(|k| {
                                let kw_lower = k.to_lowercase();
                                if query_lower.contains(&kw_lower)
                                    || kw_lower.contains(&query_lower)
                                {
                                    0.15
                                } else {
                                    0.0
                                }
                            })
                            .unwrap_or(0.0);
                        total_boost += kw_score;

                        projects.push(LinkedEntity {
                            id: proj.id,
                            name: proj.name,
                            relation: edge.relation.clone(),
                            weight: edge.weight + kw_score,
                        });
                    }
                }
                "company" => {
                    if let Ok(Some(comp)) = self.graph.get_company(connected_id) {
                        companies.push(LinkedEntity {
                            id: comp.id,
                            name: comp.name,
                            relation: edge.relation.clone(),
                            weight: edge.weight,
                        });
                    }
                }
                _ => {}
            }
        }

        // Normalize edge boost to 0.0–1.0 range
        // With 5+ strong edges you'd get ~1.0 boost
        let normalized_boost = (total_boost / 5.0).min(1.0);
        (normalized_boost, skills, projects, companies)
    }

    /// Extract key terms from the query that matched the story's fields
    fn extract_matched_terms(&self, query: &str, story: &StarStoryEntity) -> Vec<String> {
        let query_lower = query.to_lowercase();
        let mut terms = Vec::new();

        let story_fields = vec![
            story.title.as_deref().unwrap_or_default(),
            &story.situation,
            &story.task,
            &story.action,
            &story.result,
        ];

        for field in &story_fields {
            let field_lower = field.to_lowercase();
            for word in query_lower.split_whitespace() {
                if word.len() > 3 && field_lower.contains(word) {
                    if !terms.contains(&word.to_string()) {
                        terms.push(word.to_string());
                    }
                }
            }
        }

        // Check tags
        let tags = parse_tags_json(&story.tags);
        for tag in &tags {
            if query_lower.contains(tag) || tag.contains(&query_lower) {
                if !terms.contains(&tag.clone()) {
                    terms.push(tag.clone());
                }
            }
        }

        terms
    }
}

// ── Utility functions ────────────────────────────────────────────────────

/// Parse a JSON array string like `["leadership", "backend"]` into Vec<String>
fn parse_tags_json(tags: &Option<String>) -> Vec<String> {
    tags.as_deref()
        .map(|t| {
            let trimmed = t.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                trimmed[1..trimmed.len() - 1]
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').to_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect()
            } else {
                t.split(',')
                    .map(|s| s.trim().to_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect()
            }
        })
        .unwrap_or_default()
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::graph::KnowledgeGraph;

    fn setup_graph() -> KnowledgeGraph {
        let graph = KnowledgeGraph::open_in_memory().unwrap();
        // Add some skills
        let go = graph
            .create_skill("Go", Some("backend"), "expert", 5, None)
            .unwrap();
        let redis = graph
            .create_skill("Redis", Some("caching"), "advanced", 3, None)
            .unwrap();
        let k8s = graph
            .create_skill("Kubernetes", Some("devops"), "intermediate", 2, None)
            .unwrap();

        // Create a project
        let api_gw = graph
            .create_project(
                "API Gateway",
                Some("High-perf reverse proxy with Redis caching"),
                None,
                Some("go,redis,caching,backend"),
                None,
                None,
            )
            .unwrap();

        // Create STAR stories
        let perf_story = graph
            .create_star_story(
                Some("Performance Rescue"),
                "API latency of 2s causing timeouts and cart abandonment",
                "Reduce p99 latency to under 200ms without full architecture rewrite",
                "Implemented Redis caching layer, optimized N+1 queries, added connection pooling",
                "180ms p99 latency, 40% infrastructure cost reduction, zero downtime migration",
                Some(r#"["performance","backend","caching","redis","optimization"]"#),
                Some("hard"),
                Some("high"),
            )
            .unwrap();

        let lead_story = graph
            .create_star_story(
                Some("Team Leadership Crisis"),
                "Team lost the tech lead during a critical sprint with 2-week deadline",
                "Had to take ownership and coordinate without formal authority",
                "Organized daily standups, pair programming sessions, and clear task decomposition",
                "Delivered 2 weeks early with zero production bugs, team morale improved",
                Some(r#"["leadership","team-management","crisis","agile"]"#),
                Some("medium"),
                Some("high"),
            )
            .unwrap();

        let infra_story = graph
            .create_star_story(
                Some("Infrastructure Migration"),
                "Company needed to migrate from monolith to microservices on Kubernetes",
                "Design and execute migration strategy with zero downtime",
                "Created gradual migration plan, implemented canary deployments, set up monitoring",
                "Successful migration over 3 months, 99.99% uptime, 30% cost reduction",
                Some(r#"["devops","kubernetes","migration","architecture"]"#),
                Some("hard"),
                Some("critical"),
            )
            .unwrap();

        // Link edges
        graph
            .add_edge(
                &perf_story.id,
                EntityType::StarStory,
                &redis.id,
                EntityType::Skill,
                "required_skill",
                0.9,
            )
            .unwrap();
        graph
            .add_edge(
                &perf_story.id,
                EntityType::StarStory,
                &go.id,
                EntityType::Skill,
                "used_skill",
                0.7,
            )
            .unwrap();
        graph
            .add_edge(
                &perf_story.id,
                EntityType::StarStory,
                &api_gw.id,
                EntityType::Project,
                "demonstrates",
                0.9,
            )
            .unwrap();
        graph
            .add_edge(
                &lead_story.id,
                EntityType::StarStory,
                &k8s.id,
                EntityType::Skill,
                "related_skill",
                0.3,
            )
            .unwrap();

        (_, _, _, _, _) = (go, redis, k8s, api_gw, lead_story);
        graph
    }

    #[test]
    fn test_parse_tags_json() {
        let tags = Some(r#"["performance","caching","backend"]"#.to_string());
        let parsed = parse_tags_json(&tags);
        assert_eq!(parsed.len(), 3);
        assert!(parsed.contains(&"performance".to_string()));
        assert!(parsed.contains(&"caching".to_string()));
    }

    #[test]
    fn test_parse_tags_empty() {
        assert!(parse_tags_json(&None).is_empty());
        assert!(parse_tags_json(&Some("".to_string())).is_empty());
    }

    #[test]
    fn test_match_stories_performance_query() {
        let graph = setup_graph();
        let matcher = StarMatcher::new(&graph);

        let results = matcher
            .match_stories(
                "Tell me about a time you improved system performance and reduced latency",
                None,
            )
            .unwrap();

        assert!(!results.is_empty(), "Should find at least one story");
        let top = &results[0];
        assert!(
            top.relevance_score >= 0.15,
            "Top story should have meaningful score: {}",
            top.relevance_score
        );
        assert_eq!(top.story.title.as_deref(), Some("Performance Rescue"));
        // Should have linked skills (Redis, Go)
        assert!(!top.linked_skills.is_empty());
        // Should have a linked project
        assert!(!top.linked_projects.is_empty());
    }

    #[test]
    fn test_match_stories_leadership_query() {
        let graph = setup_graph();
        let matcher = StarMatcher::new(&graph);

        let results = matcher
            .match_stories(
                "Describe a situation where you had to lead a team through a difficult time",
                None,
            )
            .unwrap();

        assert!(!results.is_empty(), "Should find leadership story");
        assert_eq!(
            results[0].story.title.as_deref(),
            Some("Team Leadership Crisis")
        );
    }

    #[test]
    fn test_match_stories_empty_context() {
        let graph = setup_graph();
        let matcher = StarMatcher::new(&graph);

        let results = matcher.match_stories("", None).unwrap();
        assert!(results.is_empty(), "Empty context should return no results");
    }

    #[test]
    fn test_match_by_tags() {
        let graph = setup_graph();
        let matcher = StarMatcher::new(&graph);

        let results = matcher
            .match_by_tags(&["performance".to_string(), "caching".to_string()])
            .unwrap();
        assert!(
            !results.is_empty(),
            "Should find performance stories by tag"
        );
    }

    #[test]
    fn test_record_usage() {
        let graph = setup_graph();
        let matcher = StarMatcher::new(&graph);
        let stories = graph.list_star_stories().unwrap();
        let id = stories[0].id.clone();

        let count_before = stories[0].usage_count;
        matcher.record_usage(&id).unwrap();
        let story_after = graph.get_star_story(&id).unwrap().unwrap();
        assert_eq!(story_after.usage_count, count_before + 1);
    }

    #[test]
    fn test_edge_context_skills() {
        let graph = setup_graph();
        let matcher = StarMatcher::new(&graph);
        let stories = graph.list_star_stories().unwrap();

        // Find the performance story (should have Redis + Go linked)
        let perf = stories
            .iter()
            .find(|s| s.title.as_deref() == Some("Performance Rescue"))
            .unwrap();

        let (_, skills, projects, companies) =
            matcher.compute_edge_context(perf, "latency optimization");
        assert!(!skills.is_empty(), "Should have linked skills");
        assert!(!projects.is_empty(), "Should have linked projects");
        assert!(companies.is_empty(), "Should have no linked companies");
    }

    #[test]
    fn test_match_stories_with_options() {
        let graph = setup_graph();
        let matcher = StarMatcher::new(&graph);

        let opts = StarMatchOptions {
            max_results: 2,
            min_score: 0.5,
            ..Default::default()
        };

        let results = matcher
            .match_stories("performance latency optimization", Some(opts))
            .unwrap();
        assert!(results.len() <= 2, "Should respect max_results limit");
    }
}
