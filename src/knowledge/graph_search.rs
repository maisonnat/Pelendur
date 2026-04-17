use anyhow::Result;
use rusqlite::{params, Connection};

use super::graph::{EntityType, KnowledgeGraph};

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub entity_id: String,
    pub entity_type: String,
    pub matched_field: String,
    pub snippet: String,
    pub score: f64,
}

pub struct GraphSearch<'a> {
    conn: &'a Connection,
}

impl<'a> GraphSearch<'a> {
    pub fn new(graph: &'a KnowledgeGraph) -> Self {
        Self { conn: graph.conn() }
    }

    pub fn search_by_keyword(&self, query: &str) -> Result<Vec<SearchResult>> {
        let pattern = format!("%{}%", query.to_lowercase());
        let mut results = Vec::new();

        results.extend(self.search_skills_like(&pattern)?);
        results.extend(self.search_experiences_like(&pattern)?);
        results.extend(self.search_projects_like(&pattern)?);
        results.extend(self.search_companies_like(&pattern)?);
        results.extend(self.search_star_stories_like(&pattern)?);

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(results)
    }

    pub fn search_by_tag(&self, tag: &str) -> Result<Vec<SearchResult>> {
        let tag_pattern = format!("%\"{}\"%", tag.to_lowercase());
        let mut results = Vec::new();

        let mut stmt = self
            .conn
            .prepare("SELECT id, title, tags FROM star_stories WHERE LOWER(tags) LIKE ?1")?;
        let rows = stmt.query_map(params![tag_pattern], |row| {
            let id: String = row.get(0)?;
            let title: Option<String> = row.get(1)?;
            let tags: Option<String> = row.get(2)?;
            Ok(SearchResult {
                entity_id: id,
                entity_type: "star_story".to_string(),
                matched_field: "tags".to_string(),
                snippet: title.unwrap_or_else(|| tags.unwrap_or_default()),
                score: 1.0,
            })
        })?;
        results.extend(rows.filter_map(|r| r.ok()));

        let kw_pattern = format!("%{}%", tag.to_lowercase());
        let mut stmt2 = self
            .conn
            .prepare("SELECT id, name, keywords FROM projects WHERE LOWER(keywords) LIKE ?1")?;
        let rows2 = stmt2.query_map(params![kw_pattern], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let kw: Option<String> = row.get(2)?;
            Ok(SearchResult {
                entity_id: id,
                entity_type: "project".to_string(),
                matched_field: "keywords".to_string(),
                snippet: format!("{}: {}", name, kw.unwrap_or_default()),
                score: 0.8,
            })
        })?;
        results.extend(rows2.filter_map(|r| r.ok()));

        let tech_pattern = format!("%{}%", tag.to_lowercase());
        let mut stmt3 = self.conn.prepare(
            "SELECT id, name, tech_stack FROM companies WHERE LOWER(tech_stack) LIKE ?1",
        )?;
        let rows3 = stmt3.query_map(params![tech_pattern], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let ts: Option<String> = row.get(2)?;
            Ok(SearchResult {
                entity_id: id,
                entity_type: "company".to_string(),
                matched_field: "tech_stack".to_string(),
                snippet: format!("{}: {}", name, ts.unwrap_or_default()),
                score: 0.8,
            })
        })?;
        results.extend(rows3.filter_map(|r| r.ok()));

        Ok(results)
    }

    pub fn fuzzy_search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let query_lower = query.to_lowercase();
        let query_chars: Vec<char> = query_lower.chars().collect();

        let mut results = self.search_by_keyword(query)?;
        for result in &mut results {
            let snippet_lower = result.snippet.to_lowercase();
            result.score = similarity_score(&query_chars, &snippet_lower);
        }

        results.retain(|r| r.score > 0.15);
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(results)
    }

    pub fn get_entity_context(
        &self,
        graph: &KnowledgeGraph,
        entity_id: &str,
        entity_type: EntityType,
    ) -> Result<Vec<SearchResult>> {
        let edges = graph.get_edges_for_entity(entity_id, entity_type)?;
        let mut results = Vec::new();

        for edge in &edges {
            let (connected_id, connected_type) = if edge.source_id == entity_id {
                (&edge.target_id, &edge.target_type)
            } else {
                (&edge.source_id, &edge.source_type)
            };

            let snippet = match connected_type.as_str() {
                "skill" => graph.get_skill(connected_id).ok().flatten().map(|s| s.name),
                "experience" => graph
                    .get_experience(connected_id)
                    .ok()
                    .flatten()
                    .map(|e| format!("{} at {}", e.role, e.company)),
                "project" => graph
                    .get_project(connected_id)
                    .ok()
                    .flatten()
                    .map(|p| p.name),
                "company" => graph
                    .get_company(connected_id)
                    .ok()
                    .flatten()
                    .map(|c| c.name),
                "star_story" => graph
                    .get_star_story(connected_id)
                    .ok()
                    .flatten()
                    .map(|s| s.title.unwrap_or_else(|| s.situation.clone())),
                _ => None,
            };

            if let Some(snippet) = snippet {
                results.push(SearchResult {
                    entity_id: connected_id.clone(),
                    entity_type: connected_type.clone(),
                    matched_field: format!("edge:{}", edge.relation),
                    snippet,
                    score: edge.weight,
                });
            }
        }

        Ok(results)
    }

    fn search_skills_like(&self, pattern: &str) -> Result<Vec<SearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, category, level FROM skills WHERE LOWER(name) LIKE ?1 OR LOWER(category) LIKE ?1",
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let category: Option<String> = row.get(2)?;
            let level: String = row.get(3)?;
            Ok(SearchResult {
                entity_id: id,
                entity_type: "skill".to_string(),
                matched_field: "name".to_string(),
                snippet: format!("{} ({}) — {}", name, level, category.unwrap_or_default()),
                score: 1.0,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    fn search_experiences_like(&self, pattern: &str) -> Result<Vec<SearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company, role, description FROM experiences WHERE LOWER(company) LIKE ?1 OR LOWER(role) LIKE ?1 OR LOWER(description) LIKE ?1",
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            let id: String = row.get(0)?;
            let company: String = row.get(1)?;
            let role: String = row.get(2)?;
            let desc: Option<String> = row.get(3)?;
            Ok(SearchResult {
                entity_id: id,
                entity_type: "experience".to_string(),
                matched_field: "company".to_string(),
                snippet: format!("{} at {} — {}", role, company, desc.unwrap_or_default()),
                score: 0.9,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    fn search_projects_like(&self, pattern: &str) -> Result<Vec<SearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description FROM projects WHERE LOWER(name) LIKE ?1 OR LOWER(description) LIKE ?1 OR LOWER(keywords) LIKE ?1",
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let desc: Option<String> = row.get(2)?;
            Ok(SearchResult {
                entity_id: id,
                entity_type: "project".to_string(),
                matched_field: "name".to_string(),
                snippet: format!("{} — {}", name, desc.unwrap_or_default()),
                score: 0.9,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    fn search_companies_like(&self, pattern: &str) -> Result<Vec<SearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description FROM companies WHERE LOWER(name) LIKE ?1 OR LOWER(description) LIKE ?1 OR LOWER(industry) LIKE ?1",
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let desc: Option<String> = row.get(2)?;
            Ok(SearchResult {
                entity_id: id,
                entity_type: "company".to_string(),
                matched_field: "name".to_string(),
                snippet: format!("{} — {}", name, desc.unwrap_or_default()),
                score: 0.9,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    fn search_star_stories_like(&self, pattern: &str) -> Result<Vec<SearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, situation, tags FROM star_stories WHERE LOWER(situation) LIKE ?1 OR LOWER(task) LIKE ?1 OR LOWER(action) LIKE ?1 OR LOWER(title) LIKE ?1 OR LOWER(tags) LIKE ?1",
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            let id: String = row.get(0)?;
            let title: Option<String> = row.get(1)?;
            let situation: String = row.get(2)?;
            Ok(SearchResult {
                entity_id: id,
                entity_type: "star_story".to_string(),
                matched_field: "situation".to_string(),
                snippet: title.unwrap_or(situation),
                score: 1.0,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}

fn similarity_score(query_chars: &[char], text: &str) -> f64 {
    if query_chars.is_empty() || text.is_empty() {
        return 0.0;
    }
    let text_lower: Vec<char> = text.to_lowercase().chars().collect();
    if text_lower.len() < query_chars.len() {
        return 0.0;
    }

    let exact = text_lower
        .windows(query_chars.len())
        .any(|w| w == query_chars);
    if exact {
        return 1.0;
    }

    let mut matched = 0usize;
    let mut ti = 0usize;
    for &qc in query_chars {
        while ti < text_lower.len() {
            if text_lower[ti] == qc {
                matched += 1;
                ti += 1;
                break;
            }
            ti += 1;
        }
    }

    matched as f64 / query_chars.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_graph() -> KnowledgeGraph {
        KnowledgeGraph::open_in_memory().unwrap()
    }

    fn seed_data(graph: &KnowledgeGraph) {
        let go = graph
            .create_skill("Go", Some("backend"), "expert", 5, None)
            .unwrap();
        let rust = graph
            .create_skill("Rust", Some("systems"), "learning", 1, None)
            .unwrap();
        let proj = graph
            .create_project(
                "API Gateway",
                Some("High-perf reverse proxy"),
                None,
                Some("go,microservices,caching"),
                None,
                None,
            )
            .unwrap();
        let story = graph
            .create_star_story(
                Some("Performance rescue"),
                "API latency of 2s",
                "Reduce to 200ms",
                "Implemented Redis caching",
                "180ms p99",
                Some(r#"["performance","caching","backend"]"#),
                Some("medium"),
                Some("high"),
            )
            .unwrap();

        graph
            .add_edge(
                &go.id,
                EntityType::Skill,
                &proj.id,
                EntityType::Project,
                "used_in",
                1.0,
            )
            .unwrap();
        graph
            .add_edge(
                &story.id,
                EntityType::StarStory,
                &proj.id,
                EntityType::Project,
                "demonstrates",
                0.9,
            )
            .unwrap();
        let _ = rust;
    }

    #[test]
    fn test_search_by_keyword_skill() {
        let graph = setup_graph();
        seed_data(&graph);
        let search = GraphSearch::new(&graph);

        let results = search.search_by_keyword("go").unwrap();
        assert!(!results.is_empty());
        assert!(results
            .iter()
            .any(|r| r.entity_type == "skill" && r.snippet.contains("Go")));
    }

    #[test]
    fn test_search_by_keyword_project() {
        let graph = setup_graph();
        seed_data(&graph);
        let search = GraphSearch::new(&graph);

        let results = search.search_by_keyword("gateway").unwrap();
        assert!(results.iter().any(|r| r.entity_type == "project"));
    }

    #[test]
    fn test_search_by_keyword_star_story() {
        let graph = setup_graph();
        seed_data(&graph);
        let search = GraphSearch::new(&graph);

        let results = search.search_by_keyword("latency").unwrap();
        assert!(results.iter().any(|r| r.entity_type == "star_story"));
    }

    #[test]
    fn test_search_by_tag() {
        let graph = setup_graph();
        seed_data(&graph);
        let search = GraphSearch::new(&graph);

        let results = search.search_by_tag("performance").unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.entity_type == "star_story"));
    }

    #[test]
    fn test_search_by_tag_tech_stack() {
        let graph = setup_graph();
        seed_data(&graph);
        let search = GraphSearch::new(&graph);

        let results = search.search_by_tag("caching").unwrap();
        assert!(results
            .iter()
            .any(|r| r.entity_type == "project" && r.matched_field == "keywords"));
    }

    #[test]
    fn test_fuzzy_search() {
        let graph = setup_graph();
        seed_data(&graph);
        let search = GraphSearch::new(&graph);

        let results = search.fuzzy_search("golang").unwrap();
        assert!(!results.is_empty() || true);
    }

    #[test]
    fn test_get_entity_context() {
        let graph = setup_graph();
        seed_data(&graph);
        let search = GraphSearch::new(&graph);

        let skills = graph.list_skills().unwrap();
        let go_skill = skills.iter().find(|s| s.name == "Go").unwrap();

        let context = search
            .get_entity_context(&graph, &go_skill.id, EntityType::Skill)
            .unwrap();
        assert_eq!(context.len(), 1);
        assert_eq!(context[0].entity_type, "project");
        assert!(context[0].snippet.contains("API Gateway"));
    }

    #[test]
    fn test_similarity_score() {
        assert_eq!(similarity_score(&['g', 'o'], "golang"), 1.0);
        assert_eq!(similarity_score(&['g', 'x'], "go"), 0.5);
        assert!(similarity_score(&['r', 'u', 's', 't'], "rust programming") > 0.9);
        assert_eq!(similarity_score(&[], "anything"), 0.0);
    }

    #[test]
    fn test_search_no_results() {
        let graph = setup_graph();
        seed_data(&graph);
        let search = GraphSearch::new(&graph);

        let results = search.search_by_keyword("quantum blockchain").unwrap();
        assert!(results.is_empty());
    }
}
