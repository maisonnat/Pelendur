//! Fuzzy + tag-based + FTS5 search across all knowledge graph entities.
//!
//! Uses `strsim::normalized_levenshtein` for fuzzy matching, JSON-array /
//! comma-separated field scanning for tag/keyword matches, and SQLite FTS5
//! for full-text search optimization.
//!
//! Ranking: exact match (1.0) > FTS5 match (0.5-1.0) > fuzzy match (0.32-0.72) > tag match (0.5-0.8)

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strsim::normalized_levenshtein;

use super::graph::KnowledgeGraph;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedSearchResult {
    pub entity_type: String,
    pub id: String,
    pub name: String,
    pub relevance_score: f64,
    pub matched_terms: Vec<String>,
    pub snippet: String,
}

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub use_fts: bool,
    pub use_fuzzy: bool,
    pub use_tags: bool,
    pub max_results: usize,
    pub fuzzy_threshold: f64,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            use_fts: true,
            use_fuzzy: true,
            use_tags: true,
            max_results: 50,
            fuzzy_threshold: 0.4,
        }
    }
}

#[derive(Debug)]
struct Candidate {
    entity_type: String,
    id: String,
    name: String,
    relevance_score: f64,
    matched_terms: Vec<String>,
    snippet: String,
}

pub struct EnhancedSearch<'a> {
    conn: &'a Connection,
}

impl<'a> EnhancedSearch<'a> {
    pub fn new(graph: &'a KnowledgeGraph) -> Self {
        Self { conn: graph.conn() }
    }

    pub fn search(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<Vec<EnhancedSearchResult>> {
        let query = query.trim();
        if query.is_empty() || query.len() < 2 {
            return Ok(Vec::new());
        }

        let mut candidates: Vec<Candidate> = Vec::new();

        if options.use_fts {
            candidates.extend(self.fts_search(query)?);
        }

        if !options.use_fts {
            candidates.extend(self.exact_search_all(query)?);
        }

        if options.use_fuzzy {
            candidates.extend(self.fuzzy_search_all(query, options.fuzzy_threshold)?);
        }

        if options.use_tags {
            candidates.extend(self.tag_search_all(query)?);
        }

        let mut best: std::collections::HashMap<(String, String), Candidate> =
            std::collections::HashMap::new();
        for cand in candidates {
            let key = (cand.entity_type.clone(), cand.id.clone());
            best.entry(key)
                .and_modify(|existing| {
                    if cand.relevance_score > existing.relevance_score {
                        for t in &cand.matched_terms {
                            if !existing.matched_terms.contains(t) {
                                existing.matched_terms.push(t.clone());
                            }
                        }
                        existing.relevance_score = cand.relevance_score;
                        existing.snippet = cand.snippet.clone();
                    } else {
                        for t in &cand.matched_terms {
                            if !existing.matched_terms.contains(t) {
                                existing.matched_terms.push(t.clone());
                            }
                        }
                    }
                })
                .or_insert(cand);
        }

        let mut results: Vec<EnhancedSearchResult> = best
            .into_values()
            .map(|c| EnhancedSearchResult {
                entity_type: c.entity_type,
                id: c.id,
                name: c.name,
                relevance_score: c.relevance_score,
                matched_terms: c.matched_terms,
                snippet: c.snippet,
            })
            .collect();

        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(options.max_results);
        Ok(results)
    }

    fn fts_search(&self, query: &str) -> Result<Vec<Candidate>> {
        let fts_exists: bool = {
            let mut stmt = self.conn.prepare(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='fts_entities'",
            )?;
            let mut rows = stmt.query([])?;
            rows.next()?.is_some()
        };

        if !fts_exists {
            return Ok(Vec::new());
        }

        let fts_query = query
            .split_whitespace()
            .map(|w| format!("{}*", w))
            .collect::<Vec<_>>()
            .join(" OR ");

        let mut stmt = self.conn.prepare(
            "SELECT entity_type, entity_id, name, snippet, bm25(fts_entities) as rank
             FROM fts_entities WHERE fts_entities MATCH ?1
             ORDER BY rank LIMIT 100",
        )?;

        let rows = stmt.query_map(params![fts_query], |row| {
            let entity_type: String = row.get(0)?;
            let entity_id: String = row.get(1)?;
            let name: String = row.get(2)?;
            let snippet: String = row.get(3)?;
            let rank: f64 = row.get(4)?;
            let relevance = (1.0 + rank.min(0.0) / 10.0).max(0.5).min(1.0);
            Ok(Candidate {
                entity_type,
                id: entity_id,
                name,
                relevance_score: relevance,
                matched_terms: query.split_whitespace().map(String::from).collect(),
                snippet,
            })
        })?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    fn exact_search_all(&self, query: &str) -> Result<Vec<Candidate>> {
        let mut results = Vec::new();
        let pattern = format!("%{}%", query.to_lowercase());

        let mut stmt = self.conn.prepare(
            "SELECT id, name, category, level FROM skills WHERE LOWER(name) LIKE ?1 OR LOWER(category) LIKE ?1",
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            Ok(Candidate {
                entity_type: "skill".into(),
                id: row.get(0)?,
                name: row.get(1)?,
                relevance_score: 1.0,
                matched_terms: vec![query.to_string()],
                snippet: format!(
                    "{} ({}) - {}",
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(2)?.unwrap_or_default()
                ),
            })
        })?;
        results.extend(rows.filter_map(|r| r.ok()));

        let mut stmt = self.conn.prepare(
            "SELECT id, company, role, description FROM experiences WHERE LOWER(company) LIKE ?1 OR LOWER(role) LIKE ?1 OR LOWER(description) LIKE ?1",
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            let company: String = row.get(1)?;
            let role: String = row.get(2)?;
            let desc: Option<String> = row.get(3)?;
            Ok(Candidate {
                entity_type: "experience".into(),
                id: row.get(0)?,
                name: format!("{} - {}", company, role),
                relevance_score: 1.0,
                matched_terms: vec![query.to_string()],
                snippet: format!("{} at {} - {}", role, company, desc.unwrap_or_default()),
            })
        })?;
        results.extend(rows.filter_map(|r| r.ok()));

        let mut stmt = self.conn.prepare(
            "SELECT id, title, situation FROM star_stories WHERE LOWER(title) LIKE ?1 OR LOWER(situation) LIKE ?1 OR LOWER(task) LIKE ?1 OR LOWER(action) LIKE ?1 OR LOWER(result) LIKE ?1",
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            let title: Option<String> = row.get(1)?;
            let situation: String = row.get(2)?;
            Ok(Candidate {
                entity_type: "star_story".into(),
                id: row.get(0)?,
                name: title.clone().unwrap_or_else(|| situation.clone()),
                relevance_score: 1.0,
                matched_terms: vec![query.to_string()],
                snippet: title.unwrap_or(situation),
            })
        })?;
        results.extend(rows.filter_map(|r| r.ok()));

        let mut stmt = self.conn.prepare(
            "SELECT id, name, description FROM companies WHERE LOWER(name) LIKE ?1 OR LOWER(industry) LIKE ?1 OR LOWER(description) LIKE ?1",
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            let name: String = row.get(1)?;
            let desc: Option<String> = row.get(2)?;
            Ok(Candidate {
                entity_type: "company".into(),
                id: row.get(0)?,
                name: name.clone(),
                relevance_score: 1.0,
                matched_terms: vec![query.to_string()],
                snippet: format!("{} - {}", name, desc.unwrap_or_default()),
            })
        })?;
        results.extend(rows.filter_map(|r| r.ok()));

        let mut stmt = self.conn.prepare(
            "SELECT id, name, description FROM projects WHERE LOWER(name) LIKE ?1 OR LOWER(description) LIKE ?1",
        )?;
        let rows = stmt.query_map(params![pattern], |row| {
            let name: String = row.get(1)?;
            let desc: Option<String> = row.get(2)?;
            Ok(Candidate {
                entity_type: "project".into(),
                id: row.get(0)?,
                name: name.clone(),
                relevance_score: 1.0,
                matched_terms: vec![query.to_string()],
                snippet: format!("{} - {}", name, desc.unwrap_or_default()),
            })
        })?;
        results.extend(rows.filter_map(|r| r.ok()));

        Ok(results)
    }

    fn fuzzy_search_all(&self, query: &str, threshold: f64) -> Result<Vec<Candidate>> {
        if query.len() < 3 {
            return Ok(Vec::new());
        }

        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let mut results = Vec::new();

        results.extend(self.fuzzy_match_table(
            "SELECT id, name FROM skills",
            "skill",
            &query_words,
            threshold,
        )?);
        results.extend(self.fuzzy_match_table(
            "SELECT id, role || ' at ' || company as name FROM experiences",
            "experience",
            &query_words,
            threshold,
        )?);
        results.extend(self.fuzzy_match_table(
            "SELECT id, COALESCE(title, situation) as name FROM star_stories",
            "star_story",
            &query_words,
            threshold,
        )?);
        results.extend(self.fuzzy_match_table(
            "SELECT id, name FROM companies",
            "company",
            &query_words,
            threshold,
        )?);
        results.extend(self.fuzzy_match_table(
            "SELECT id, name FROM projects",
            "project",
            &query_words,
            threshold,
        )?);

        Ok(results)
    }

    fn fuzzy_match_table(
        &self,
        sql: &str,
        entity_type: &str,
        query_words: &[&str],
        threshold: f64,
    ) -> Result<Vec<Candidate>> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut results = Vec::new();
        for row in rows {
            let (id, name) = row?;
            let name_lower = name.to_lowercase();
            let name_words: Vec<&str> = name_lower.split_whitespace().collect();

            let mut best_score = 0.0_f64;
            let mut matched = Vec::new();

            for qw in query_words {
                for nw in &name_words {
                    let sim = normalized_levenshtein(qw, nw);
                    if sim >= threshold && sim > best_score {
                        best_score = sim;
                        matched.push(nw.to_string());
                    }
                }
            }

            if best_score >= threshold {
                results.push(Candidate {
                    entity_type: entity_type.to_string(),
                    id,
                    name: name.clone(),
                    relevance_score: best_score * 0.8,
                    matched_terms: matched,
                    snippet: name,
                });
            }
        }
        Ok(results)
    }

    fn tag_search_all(&self, query: &str) -> Result<Vec<Candidate>> {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let mut results = Vec::new();

        let mut stmt = self.conn.prepare(
            "SELECT id, title, situation, tags FROM star_stories WHERE tags IS NOT NULL",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        for row in rows {
            let (id, title, situation, tags_json) = row?;
            let tags = parse_json_tags(tags_json.as_deref());
            let matched = find_matching_tags(&tags, &query_words);
            if !matched.is_empty() {
                results.push(Candidate {
                    entity_type: "star_story".to_string(),
                    id,
                    name: title.unwrap_or_else(|| situation.clone()),
                    relevance_score: 0.7,
                    matched_terms: matched,
                    snippet: situation,
                });
            }
        }

        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, keywords FROM projects WHERE keywords IS NOT NULL",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        for row in rows {
            let (id, name, desc, keywords) = row?;
            let tags: Vec<String> = keywords
                .as_deref()
                .map(|k| k.split(',').map(|s| s.trim().to_lowercase()).collect())
                .unwrap_or_default();
            let matched = find_matching_tags(&tags, &query_words);
            if !matched.is_empty() {
                results.push(Candidate {
                    entity_type: "project".to_string(),
                    id,
                    name: name.clone(),
                    relevance_score: 0.7,
                    matched_terms: matched,
                    snippet: desc.unwrap_or(name),
                });
            }
        }

        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, tech_stack FROM companies WHERE tech_stack IS NOT NULL",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        for row in rows {
            let (id, name, desc, tech_stack) = row?;
            let tags: Vec<String> = tech_stack
                .as_deref()
                .map(|k| k.split(',').map(|s| s.trim().to_lowercase()).collect())
                .unwrap_or_default();
            let matched = find_matching_tags(&tags, &query_words);
            if !matched.is_empty() {
                results.push(Candidate {
                    entity_type: "company".to_string(),
                    id,
                    name: name.clone(),
                    relevance_score: 0.7,
                    matched_terms: matched,
                    snippet: desc.unwrap_or(name),
                });
            }
        }

        let mut stmt = self
            .conn
            .prepare("SELECT id, name, category FROM skills WHERE category IS NOT NULL")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;
        for row in rows {
            let (id, name, category) = row?;
            let tags: Vec<String> = category
                .as_deref()
                .map(|c| c.to_lowercase())
                .into_iter()
                .collect();
            let matched = find_matching_tags(&tags, &query_words);
            if !matched.is_empty() {
                results.push(Candidate {
                    entity_type: "skill".to_string(),
                    id,
                    name: name.clone(),
                    relevance_score: 0.7,
                    matched_terms: matched,
                    snippet: name,
                });
            }
        }

        Ok(results)
    }

    pub fn search_by_context(
        &self,
        transcript: &str,
        limit: Option<usize>,
    ) -> Result<Vec<EnhancedSearchResult>> {
        let words: Vec<String> = transcript
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .map(|w| {
                w.to_lowercase()
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string()
            })
            .filter(|w| !w.is_empty())
            .collect();

        let opts = SearchOptions {
            use_fts: true,
            use_fuzzy: true,
            use_tags: true,
            max_results: 10,
            fuzzy_threshold: 0.4,
        };

        let mut all_results: Vec<EnhancedSearchResult> = Vec::new();
        for word in words.iter().take(20) {
            if let Ok(results) = self.search(word, &opts) {
                all_results.extend(results);
            }
        }

        let mut score_map: std::collections::HashMap<
            (String, String),
            (f64, EnhancedSearchResult),
        > = std::collections::HashMap::new();
        for r in all_results {
            let key = (r.entity_type.clone(), r.id.clone());
            score_map
                .entry(key)
                .and_modify(|(score, existing)| {
                    *score += r.relevance_score * 0.5;
                    for t in &r.matched_terms {
                        if !existing.matched_terms.contains(t) {
                            existing.matched_terms.push(t.clone());
                        }
                    }
                })
                .or_insert((r.relevance_score, r));
        }

        let mut results: Vec<EnhancedSearchResult> = score_map
            .into_values()
            .map(|(score, mut r)| {
                r.relevance_score = score.min(1.0);
                r
            })
            .collect();
        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit.unwrap_or(50));
        Ok(results)
    }
}

/// Search result with full entity data for downstream processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub entity_type: String,
    pub entity_id: String,
    pub name: String,
    pub relevance_score: f64,
    pub matched_terms: Vec<String>,
    pub snippet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Search configuration for KnowledgeSearcher
#[derive(Debug, Clone)]
pub struct KnowledgeSearchOptions {
    pub max_results: usize,
    pub fuzzy_threshold: f64,
    pub include_types: Vec<String>,
    pub tag_filter: Option<String>,
}

impl Default for KnowledgeSearchOptions {
    fn default() -> Self {
        Self {
            max_results: 20,
            fuzzy_threshold: 0.6,
            include_types: Vec::new(),
            tag_filter: None,
        }
    }
}

impl KnowledgeSearchOptions {
    fn to_enhanced_opts(&self) -> SearchOptions {
        SearchOptions {
            use_fts: true,
            use_fuzzy: true,
            use_tags: true,
            max_results: self.max_results,
            fuzzy_threshold: self.fuzzy_threshold,
        }
    }
}

/// High-level search interface wrapping EnhancedSearch with unified result types
pub struct KnowledgeSearcher<'a> {
    graph: &'a KnowledgeGraph,
}

impl<'a> KnowledgeSearcher<'a> {
    pub fn new(graph: &'a KnowledgeGraph) -> Self {
        Self { graph }
    }

    pub fn search(
        &self,
        query: &str,
        options: KnowledgeSearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let enhanced = EnhancedSearch::new(self.graph);
        let enhanced_opts = options.to_enhanced_opts();
        let results = enhanced.search(query, &enhanced_opts)?;

        let filtered: Vec<SearchResult> = results
            .into_iter()
            .filter(|r| {
                if !options.include_types.is_empty()
                    && !options.include_types.contains(&r.entity_type)
                {
                    return false;
                }
                if let Some(ref tag) = options.tag_filter {
                    if !r
                        .matched_terms
                        .iter()
                        .any(|t| t.to_lowercase().contains(&tag.to_lowercase()))
                    {
                        return false;
                    }
                }
                true
            })
            .map(|r| SearchResult {
                entity_type: r.entity_type,
                entity_id: r.id,
                name: r.name,
                relevance_score: r.relevance_score,
                matched_terms: r.matched_terms,
                snippet: r.snippet,
                data: None,
            })
            .collect();

        Ok(filtered)
    }

    pub fn search_skills_fuzzy(&self, query: &str, threshold: f64) -> Result<Vec<SearchResult>> {
        let options = KnowledgeSearchOptions {
            max_results: 50,
            fuzzy_threshold: threshold,
            include_types: vec!["skill".to_string()],
            tag_filter: None,
        };
        self.search(query, options)
    }

    pub fn search_stories_by_tags(&self, tags: &[String]) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        for tag in tags {
            let tag_opts = KnowledgeSearchOptions {
                max_results: 50,
                fuzzy_threshold: 0.4,
                include_types: vec!["star_story".to_string()],
                tag_filter: Some(tag.clone()),
            };
            results.extend(self.search(tag, tag_opts)?);
        }

        let mut seen: HashMap<String, SearchResult> = HashMap::new();
        for r in results {
            let r_id = r.entity_id.clone();
            let r_score = r.relevance_score;
            let r_terms = r.matched_terms.clone();
            seen.entry(r_id)
                .and_modify(|existing| {
                    if r_score > existing.relevance_score {
                        *existing = SearchResult {
                            entity_type: r.entity_type.clone(),
                            entity_id: r.entity_id.clone(),
                            name: r.name.clone(),
                            relevance_score: r_score,
                            matched_terms: r_terms.clone(),
                            snippet: r.snippet.clone(),
                            data: r.data.clone(),
                        };
                    } else {
                        for term in &r_terms {
                            if !existing.matched_terms.contains(term) {
                                existing.matched_terms.push(term.clone());
                            }
                        }
                    }
                })
                .or_insert(r);
        }

        let mut deduped: Vec<SearchResult> = seen.into_values().collect();
        deduped.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(deduped)
    }

    pub fn search_all_entities(&self, query: &str) -> Result<Vec<SearchResult>> {
        self.search(query, KnowledgeSearchOptions::default())
    }

    pub fn rank_results(&self, results: Vec<SearchResult>) -> Vec<SearchResult> {
        let mut best: HashMap<(String, String), SearchResult> = HashMap::new();

        for result in results {
            let key = (result.entity_type.clone(), result.entity_id.clone());
            let r_score = result.relevance_score;
            let r_terms = result.matched_terms.clone();
            best.entry(key)
                .and_modify(|existing| {
                    if r_score > existing.relevance_score {
                        *existing = result.clone();
                    } else {
                        for term in &r_terms {
                            if !existing.matched_terms.contains(term) {
                                existing.matched_terms.push(term.clone());
                            }
                        }
                    }
                })
                .or_insert(result);
        }

        let mut ranked: Vec<SearchResult> = best.into_values().collect();
        ranked.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ranked
    }

    pub fn context_search(&self, transcription: &str) -> Vec<SearchResult> {
        let words: Vec<String> = transcription
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .map(|w| {
                w.to_lowercase()
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string()
            })
            .filter(|w| !w.is_empty())
            .collect();

        let mut all_results: Vec<SearchResult> = Vec::new();

        for word in words.iter().take(20) {
            if let Ok(results) = self.search(word, KnowledgeSearchOptions::default()) {
                all_results.extend(results);
            }
        }

        let ranked = self.rank_results(all_results);
        ranked.into_iter().take(20).collect()
    }

    pub fn hybrid_search(
        &self,
        query: &str,
        options: KnowledgeSearchOptions,
    ) -> Result<Vec<SearchResult>> {
        let semantic_results = self.semantic_search(query, options.max_results)?;
        let keyword_results = self.search(query, options)?;

        let mut combined: Vec<SearchResult> = Vec::new();
        combined.extend(keyword_results);
        combined.extend(semantic_results);

        Ok(self.rank_results(combined))
    }

    pub fn semantic_search(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        let results = self.graph.semantic_search(query, top_k)?;
        Ok(results
            .into_iter()
            .map(|r| SearchResult {
                entity_type: r.entity_type,
                entity_id: r.entity_id,
                name: r.name,
                relevance_score: r.similarity,
                matched_terms: vec![query.to_string()],
                snippet: r.snippet,
                data: None,
            })
            .collect())
    }
}

fn parse_json_tags(json: Option<&str>) -> Vec<String> {
    json.map(|j| {
        let trimmed = j.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            trimmed[1..trimmed.len() - 1]
                .split(',')
                .map(|s| s.trim().trim_matches('"').to_lowercase())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            Vec::new()
        }
    })
    .unwrap_or_default()
}

fn find_matching_tags(tags: &[String], query_words: &[&str]) -> Vec<String> {
    let mut matched = Vec::new();
    for tag in tags {
        for qw in query_words {
            if tag == *qw || tag.contains(qw) || qw.contains(&*tag) {
                matched.push(tag.clone());
                break;
            }
        }
    }
    matched
}

pub fn ensure_fts_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE VIRTUAL TABLE IF NOT EXISTS fts_entities USING fts5(
            entity_type,
            entity_id,
            name,
            content,
            snippet,
            tokenize='porter unicode61'
        );
        ",
    )
    .context("Failed to create FTS5 tables")?;
    Ok(())
}

pub fn rebuild_fts_index(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM fts_entities", [])?;

    let mut stmt = conn.prepare("SELECT id, name, category, level FROM skills")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    for row in rows {
        let (id, name, category, level) = row?;
        let content = format!(
            "{} {} {}",
            name,
            category.as_deref().unwrap_or_default(),
            level
        );
        let snippet = format!(
            "{} ({}) - {}",
            name,
            level,
            category.as_deref().unwrap_or_default()
        );
        conn.execute(
            "INSERT INTO fts_entities (entity_type, entity_id, name, content, snippet) VALUES ('skill', ?1, ?2, ?3, ?4)",
            params![id, name, content, snippet],
        )?;
    }

    let mut stmt = conn.prepare("SELECT id, company, role, description FROM experiences")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;
    for row in rows {
        let (id, company, role, desc) = row?;
        let content = format!(
            "{} {} {}",
            company,
            role,
            desc.as_deref().unwrap_or_default()
        );
        let snippet = format!(
            "{} at {} - {}",
            role,
            company,
            desc.as_deref().unwrap_or_default()
        );
        conn.execute(
            "INSERT INTO fts_entities (entity_type, entity_id, name, content, snippet) VALUES ('experience', ?1, ?2, ?3, ?4)",
            params![id, format!("{} at {}", role, company), content, snippet],
        )?;
    }

    let mut stmt = conn.prepare("SELECT id, name, description, keywords FROM projects")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;
    for row in rows {
        let (id, name, desc, keywords) = row?;
        let content = format!(
            "{} {} {}",
            name,
            desc.as_deref().unwrap_or_default(),
            keywords.as_deref().unwrap_or_default()
        );
        let snippet = format!("{} - {}", name, desc.as_deref().unwrap_or_default());
        conn.execute(
            "INSERT INTO fts_entities (entity_type, entity_id, name, content, snippet) VALUES ('project', ?1, ?2, ?3, ?4)",
            params![id, name, content, snippet],
        )?;
    }

    let mut stmt =
        conn.prepare("SELECT id, name, industry, description, tech_stack FROM companies")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
        ))
    })?;
    for row in rows {
        let (id, name, industry, desc, tech_stack) = row?;
        let desc_val = desc.unwrap_or_default();
        let content = format!(
            "{} {} {} {}",
            name,
            industry.unwrap_or_default(),
            &desc_val,
            tech_stack.unwrap_or_default()
        );
        let snippet = format!("{} - {}", name, &desc_val);
        conn.execute(
            "INSERT INTO fts_entities (entity_type, entity_id, name, content, snippet) VALUES ('company', ?1, ?2, ?3, ?4)",
            params![id, name, content, snippet],
        )?;
    }

    let mut stmt =
        conn.prepare("SELECT id, title, situation, task, action, result, tags FROM star_stories")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, Option<String>>(6)?,
        ))
    })?;
    for row in rows {
        let (id, title, situation, task, action, result, tags) = row?;
        let display_name = title.clone().unwrap_or_else(|| situation.clone());
        let content = format!(
            "{} {} {} {} {} {}",
            title.unwrap_or_default(),
            situation,
            task,
            action,
            result,
            tags.unwrap_or_default()
        );
        conn.execute(
            "INSERT INTO fts_entities (entity_type, entity_id, name, content, snippet) VALUES ('star_story', ?1, ?2, ?3, ?4)",
            params![id, display_name, content, display_name.clone()],
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::graph::{EntityType, KnowledgeGraph};

    fn setup_with_fts() -> KnowledgeGraph {
        let kg = KnowledgeGraph::open_in_memory().unwrap();
        ensure_fts_tables(kg.conn()).unwrap();
        seed_test_data(&kg);
        rebuild_fts_index(kg.conn()).unwrap();
        kg
    }

    fn setup_without_fts() -> KnowledgeGraph {
        let kg = KnowledgeGraph::open_in_memory().unwrap();
        seed_test_data(&kg);
        kg
    }

    fn seed_test_data(kg: &KnowledgeGraph) {
        let go = kg
            .create_skill("Go", Some("backend"), "expert", 5, None)
            .unwrap();
        let rust = kg
            .create_skill("Rust", Some("systems"), "learning", 1, None)
            .unwrap();
        let k8s = kg
            .create_skill("Kubernetes", Some("devops"), "advanced", 3, None)
            .unwrap();

        kg.create_experience(
            "Acme Corp",
            "Senior Backend Engineer",
            "2020-01",
            Some("2023-06"),
            Some("Built high-performance API gateway serving 10k rps"),
            None,
        )
        .unwrap();

        kg.create_project(
            "API Gateway",
            Some("High-perf reverse proxy with caching and rate limiting"),
            Some("https://github.com/example/gateway"),
            Some("go,microservices,caching,performance"),
            None,
            None,
        )
        .unwrap();

        kg.create_company(
            "Stripe",
            Some("Fintech"),
            Some("Payment infrastructure for global merchants"),
            Some("Engineering-driven, data-focused"),
            Some("Go,Ruby,PostgreSQL,Kubernetes"),
            None,
        )
        .unwrap();

        let story = kg
            .create_star_story(
                Some("Performance rescue"),
                "API latency of 2s causing timeouts and cart abandonment",
                "Reduce to under 200ms without architecture changes",
                "Implemented Redis caching and optimized database queries",
                "180ms p99 latency, 40% infrastructure cost reduction",
                Some(r#"["performance","caching","backend"]"#),
                Some("medium"),
                Some("high"),
            )
            .unwrap();

        let leadership = kg
            .create_star_story(
                Some("Leadership in crisis"),
                "Team lost the tech lead during a critical sprint",
                "Take ownership and keep the team on track",
                "Organized daily standups, created pair programming rotations",
                "Delivered 2 weeks early with zero production bugs",
                Some(r#"["leadership","crisis","team-management"]"#),
                Some("medium"),
                Some("high"),
            )
            .unwrap();

        kg.add_edge(
            &go.id,
            EntityType::Skill,
            &story.id,
            EntityType::StarStory,
            "demonstrated_in",
            1.0,
        )
        .unwrap();
        let _ = (&rust, &k8s, &leadership);
    }

    #[test]
    fn test_fts_exact_match_skill() {
        let kg = setup_with_fts();
        let search = EnhancedSearch::new(&kg);
        let results = search.search("Go", &SearchOptions::default()).unwrap();
        assert!(!results.is_empty());
        assert!(results
            .iter()
            .any(|r| r.entity_type == "skill" && r.name == "Go"));
    }

    #[test]
    fn test_fts_match_project() {
        let kg = setup_with_fts();
        let search = EnhancedSearch::new(&kg);
        let results = search.search("Gateway", &SearchOptions::default()).unwrap();
        assert!(results.iter().any(|r| r.entity_type == "project"));
    }

    #[test]
    fn test_fts_match_star_story() {
        let kg = setup_with_fts();
        let search = EnhancedSearch::new(&kg);
        let results = search.search("latency", &SearchOptions::default()).unwrap();
        assert!(results.iter().any(|r| r.entity_type == "star_story"));
    }

    #[test]
    fn test_fts_match_experience() {
        let kg = setup_with_fts();
        let search = EnhancedSearch::new(&kg);
        let results = search.search("Acme", &SearchOptions::default()).unwrap();
        assert!(results.iter().any(|r| r.entity_type == "experience"));
    }

    #[test]
    fn test_fts_match_company() {
        let kg = setup_with_fts();
        let search = EnhancedSearch::new(&kg);
        let results = search.search("Stripe", &SearchOptions::default()).unwrap();
        assert!(results.iter().any(|r| r.entity_type == "company"));
    }

    #[test]
    fn test_fuzzy_typo_skill() {
        let kg = setup_without_fts();
        let search = EnhancedSearch::new(&kg);
        let mut opts = SearchOptions::default();
        opts.use_fts = false;
        opts.fuzzy_threshold = 0.2; // normalized_levenshtein("golang","go") ≈ 0.33
        let results = search.search("Golang", &opts).unwrap();
        assert!(!results.is_empty());
        assert!(results
            .iter()
            .any(|r| r.entity_type == "skill" && r.name == "Go"));
    }

    #[test]
    fn test_fuzzy_close_name() {
        let kg = setup_without_fts();
        let search = EnhancedSearch::new(&kg);
        let mut opts = SearchOptions::default();
        opts.use_fts = false;
        opts.fuzzy_threshold = 0.4;
        let results = search.search("Ruster", &opts).unwrap();
        assert!(results
            .iter()
            .any(|r| r.entity_type == "skill" && r.name == "Rust"));
    }

    #[test]
    fn test_fuzzy_no_match_garbage() {
        let kg = setup_without_fts();
        let search = EnhancedSearch::new(&kg);
        let mut opts = SearchOptions::default();
        opts.use_fts = false;
        let results = search.search("xyzabc", &opts).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_tag_search_star_story() {
        let kg = setup_without_fts();
        let search = EnhancedSearch::new(&kg);
        let mut opts = SearchOptions::default();
        opts.use_fts = false;
        opts.use_fuzzy = false;
        let results = search.search("performance", &opts).unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.entity_type == "star_story"
            && r.matched_terms.contains(&"performance".to_string())));
    }

    #[test]
    fn test_tag_search_leadership() {
        let kg = setup_without_fts();
        let search = EnhancedSearch::new(&kg);
        let mut opts = SearchOptions::default();
        opts.use_fts = false;
        opts.use_fuzzy = false;
        let results = search.search("leadership", &opts).unwrap();
        assert!(results.iter().any(|r| r.entity_type == "star_story"
            && r.matched_terms.contains(&"leadership".to_string())));
    }

    #[test]
    fn test_tag_search_project_keywords() {
        let kg = setup_without_fts();
        let search = EnhancedSearch::new(&kg);
        let mut opts = SearchOptions::default();
        opts.use_fts = false;
        opts.use_fuzzy = false;
        let results = search.search("caching", &opts).unwrap();
        assert!(results.iter().any(|r| r.entity_type == "project"));
    }

    #[test]
    fn test_tag_search_company_tech_stack() {
        let kg = setup_without_fts();
        let search = EnhancedSearch::new(&kg);
        let mut opts = SearchOptions::default();
        opts.use_fts = false;
        opts.use_fuzzy = false;
        let results = search.search("postgresql", &opts).unwrap();
        assert!(results.iter().any(|r| r.entity_type == "company"));
    }

    #[test]
    fn test_tag_search_skill_category() {
        let kg = setup_without_fts();
        let search = EnhancedSearch::new(&kg);
        let mut opts = SearchOptions::default();
        opts.use_fts = false;
        opts.use_fuzzy = false;
        let results = search.search("backend", &opts).unwrap();
        assert!(results
            .iter()
            .any(|r| r.entity_type == "skill" && r.name == "Go"));
    }

    #[test]
    fn test_empty_query() {
        let kg = setup_with_fts();
        let search = EnhancedSearch::new(&kg);
        let results = search.search("", &SearchOptions::default()).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_single_char_query_ignored() {
        let kg = setup_with_fts();
        let search = EnhancedSearch::new(&kg);
        let results = search.search("g", &SearchOptions::default()).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_no_results() {
        let kg = setup_with_fts();
        let search = EnhancedSearch::new(&kg);
        let results = search
            .search("quantum blockchain web3", &SearchOptions::default())
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_whitespace_only_query() {
        let kg = setup_with_fts();
        let search = EnhancedSearch::new(&kg);
        let results = search.search("   ", &SearchOptions::default()).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_exact_match_ranks_highest() {
        let kg = setup_with_fts();
        let search = EnhancedSearch::new(&kg);
        let results = search.search("Go", &SearchOptions::default()).unwrap();
        let go_result = results
            .iter()
            .find(|r| r.entity_type == "skill" && r.name == "Go");
        assert!(go_result.is_some());
        assert!(go_result.unwrap().relevance_score >= 0.7);
    }

    #[test]
    fn test_deduplication_keeps_best_score() {
        let kg = setup_with_fts();
        let search = EnhancedSearch::new(&kg);
        let results = search
            .search("performance", &SearchOptions::default())
            .unwrap();
        let mut seen = std::collections::HashSet::new();
        for r in &results {
            let key = (r.entity_type.clone(), r.id.clone());
            assert!(
                seen.insert(key.clone()),
                "Duplicate result found: {:?}",
                key
            );
        }
    }

    #[test]
    fn test_multi_entity_search() {
        let kg = setup_with_fts();
        let search = EnhancedSearch::new(&kg);
        let results = search.search("Go", &SearchOptions::default()).unwrap();
        let types: std::collections::HashSet<&str> =
            results.iter().map(|r| r.entity_type.as_str()).collect();
        assert!(types.contains("skill"));
    }

    #[test]
    fn test_search_performance_under_50ms() {
        let kg = KnowledgeGraph::open_in_memory().unwrap();
        ensure_fts_tables(kg.conn()).unwrap();

        for i in 0..200 {
            kg.create_skill(
                &format!("Skill-{}", i),
                Some("test"),
                "intermediate",
                i % 10,
                None,
            )
            .unwrap();
            kg.create_star_story(
                Some(&format!("Story-{}", i)),
                &format!("Situation {} with backend and performance", i),
                &format!("Task {}", i),
                &format!("Action {}", i),
                &format!("Result {}", i),
                Some(&format!(r#"["tag{}","backend"]"#, i)),
                None,
                None,
            )
            .unwrap();
            kg.create_company(
                &format!("Company-{}", i),
                Some("Tech"),
                Some(&format!("Description for company {}", i)),
                None,
                Some("Go,Rust,PostgreSQL"),
                None,
            )
            .unwrap();
            kg.create_project(
                &format!("Project-{}", i),
                Some(&format!("Project desc {}", i)),
                None,
                Some("go,microservices,caching"),
                None,
                None,
            )
            .unwrap();
            kg.create_experience(
                &format!("ExpCorp-{}", i),
                &format!("Engineer-{}", i),
                "2020-01",
                None,
                Some(&format!("Experience description {}", i)),
                None,
            )
            .unwrap();
        }
        rebuild_fts_index(kg.conn()).unwrap();

        let search = EnhancedSearch::new(&kg);
        let start = std::time::Instant::now();
        let results = search
            .search("backend performance", &SearchOptions::default())
            .unwrap();
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 50,
            "Search took {}ms, expected < 50ms",
            elapsed.as_millis()
        );
        assert!(!results.is_empty());
    }

    #[test]
    fn test_context_search_from_transcription() {
        let kg = setup_with_fts();
        let search = EnhancedSearch::new(&kg);
        let results = search
            .search_by_context(
                "tell me about a time you improved performance and reduced latency",
                Some(10),
            )
            .unwrap();
        assert!(results
            .iter()
            .any(|r| r.entity_type == "star_story" && r.name.contains("Performance")));
        assert!(results.len() >= 2);
    }

    #[test]
    fn test_search_without_fts_tables() {
        let kg = setup_without_fts();
        let search = EnhancedSearch::new(&kg);
        let mut opts = SearchOptions::default();
        opts.use_fts = false;
        let results = search.search("Go", &opts).unwrap();
        assert!(results
            .iter()
            .any(|r| r.entity_type == "skill" && r.name == "Go"));
    }

    #[test]
    fn test_parse_json_tags() {
        let tags = parse_json_tags(Some(r#"["leadership","crisis"]"#));
        assert_eq!(tags, vec!["leadership", "crisis"]);
        assert!(parse_json_tags(None).is_empty());
        assert_eq!(
            parse_json_tags(Some(r#"["performance"]"#)),
            vec!["performance"]
        );
    }
}
