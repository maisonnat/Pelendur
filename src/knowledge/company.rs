use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::knowledge::graph::{CompanyEntity, KnowledgeGraph};

/// Structured company research data parsed from overview.md
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyResearch {
    pub company_name: String,
    pub industry: Option<String>,
    pub description: Option<String>,
    pub culture: Option<String>,
    pub tech_stack: Option<String>,
    pub strategic_angle: Option<String>,
    pub key_challenges: Vec<String>,
    pub products: Vec<String>,
    pub funding_rounds: Vec<FundingRound>,
    pub competitors: Vec<String>,
    pub interview_tips: Vec<String>,
    pub recent_news: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingRound {
    pub round: String,
    pub amount: Option<String>,
    pub date: Option<String>,
    pub investors: Option<String>,
}

impl CompanyResearch {
    /// Create skeleton from just a name — for templates
    pub fn new(name: &str) -> Self {
        Self {
            company_name: name.to_string(),
            industry: None,
            description: None,
            culture: None,
            tech_stack: None,
            strategic_angle: None,
            key_challenges: Vec::new(),
            products: Vec::new(),
            funding_rounds: Vec::new(),
            competitors: Vec::new(),
            interview_tips: Vec::new(),
            recent_news: Vec::new(),
        }
    }

    /// Parse overview.md into CompanyResearch
    pub fn from_markdown(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        let lines: Vec<&str> = content.lines().collect();
        let mut research = CompanyResearch::new("");

        // First line should be "# CompanyName Research"
        if let Some(first) = lines.first() {
            let name = first
                .trim_start_matches('#')
                .trim()
                .trim_end_matches("Research")
                .trim();
            research.company_name = name.to_string();
        }

        // Parse bullet-point key-value pairs
        let mut current_section = String::new();

        for line in &lines {
            let trimmed = line.trim();

            // Detect section headers
            if trimmed.starts_with("## ") {
                current_section = trimmed.trim_start_matches("## ").trim().to_lowercase();
                continue;
            }

            // Parse key-value bullets
            if let Some(val) = trimmed.strip_prefix("- **").and_then(|s| {
                s.split_once("**: ")
            }) {
                let key = val.0.trim().to_lowercase();
                let value = val.1.trim().to_string();
                match key.as_str() {
                    "industry" => research.industry = Some(value),
                    "description" => research.description = Some(value),
                    "culture" => research.culture = Some(value),
                    "tech stack" | "tech_stack" | "stack" => research.tech_stack = Some(value),
                    "strategic angle" | "strategic_angle" => research.strategic_angle = Some(value),
                    _ => {} // ignore unknown keys
                }
                continue;
            }

            // Parse list items within sections
            if trimmed.starts_with("- ") {
                let item = trimmed.trim_start_matches("- ").trim();
                if !item.is_empty() {
                    match current_section.as_str() {
                        "key challenges" | "challenges" => research.key_challenges.push(item.to_string()),
                        "products" | "product" => research.products.push(item.to_string()),
                        "competitors" | "competition" => research.competitors.push(item.to_string()),
                        "interview tips" | "tips" => research.interview_tips.push(item.to_string()),
                        "recent news" | "news" => research.recent_news.push(item.to_string()),
                        "funding" | "funding rounds" => {
                            if let Some((round, rest)) = item.split_once(':') {
                                research.funding_rounds.push(FundingRound {
                                    round: round.trim().to_string(),
                                    amount: Some(rest.trim().to_string()),
                                    date: None,
                                    investors: None,
                                });
                            }
                        }
                        _ => {} // skip unclassified
                    }
                }
            }
        }

        Ok(research)
    }

    /// Render to structured markdown
    pub fn to_markdown(&self) -> String {
        let mut md = format!("# {} Research\n\n", self.company_name);

        if let Some(v) = &self.industry {
            md.push_str(&format!("- **Industry**: {}\n", v));
        }
        if let Some(v) = &self.description {
            md.push_str(&format!("- **Description**: {}\n", v));
        }
        if let Some(v) = &self.culture {
            md.push_str(&format!("- **Culture**: {}\n", v));
        }
        if let Some(v) = &self.tech_stack {
            md.push_str(&format!("- **Tech Stack**: {}\n", v));
        }
        if let Some(v) = &self.strategic_angle {
            md.push_str(&format!("- **Strategic Angle**: {}\n", v));
        }

        if !self.key_challenges.is_empty() {
            md.push_str("\n## Key Challenges\n");
            for c in &self.key_challenges {
                md.push_str(&format!("- {}\n", c));
            }
        }

        if !self.products.is_empty() {
            md.push_str("\n## Products\n");
            for p in &self.products {
                md.push_str(&format!("- {}\n", p));
            }
        }

        if !self.competitors.is_empty() {
            md.push_str("\n## Competitors\n");
            for c in &self.competitors {
                md.push_str(&format!("- {}\n", c));
            }
        }

        if !self.funding_rounds.is_empty() {
            md.push_str("\n## Funding Rounds\n");
            for f in &self.funding_rounds {
                md.push_str(&format!("- **{}**: {}\n", f.round, f.amount.as_deref().unwrap_or("N/A")));
            }
        }

        if !self.interview_tips.is_empty() {
            md.push_str("\n## Interview Tips\n");
            for t in &self.interview_tips {
                md.push_str(&format!("- {}\n", t));
            }
        }

        if !self.recent_news.is_empty() {
            md.push_str("\n## Recent News\n");
            for n in &self.recent_news {
                md.push_str(&format!("- {}\n", n));
            }
        }

        md
    }

    /// Convert to graph entity for the knowledge graph
    pub fn to_entity(&self) -> CompanyEntity {
        CompanyEntity {
            id: String::new(), // generated by graph
            name: self.company_name.clone(),
            industry: self.industry.clone(),
            description: self.description.clone(),
            culture: self.culture.clone(),
            tech_stack: self.tech_stack.clone(),
            strategic_angle: self.strategic_angle.clone(),
        }
    }
}

/// Loader that syncs company research from markdown files into the graph DB
pub struct CompanyLoader {
    knowledge_base_path: PathBuf,
}

impl CompanyLoader {
    pub fn new(knowledge_base_path: &str) -> Self {
        Self {
            knowledge_base_path: PathBuf::from(knowledge_base_path),
        }
    }

    /// Scan `knowledge/companies/` directory and load all overview.md files
    pub fn load_all_into_graph(&self, graph: &KnowledgeGraph) -> Result<Vec<CompanyEntity>> {
        let companies_dir = self.knowledge_base_path.join("companies");
        if !companies_dir.exists() {
            return Ok(Vec::new());
        }

        let mut loaded = Vec::new();

        for entry in fs::read_dir(&companies_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let overview_path = path.join("overview.md");
            if !overview_path.exists() {
                continue;
            }

            let research = CompanyResearch::from_markdown(&overview_path)?;
            let entity = self.sync_to_graph(&research, graph)?;
            loaded.push(entity);
        }

        Ok(loaded)
    }

    /// Sync one company's research into the graph (upsert by name)
    pub fn sync_to_graph(&self, research: &CompanyResearch, graph: &KnowledgeGraph) -> Result<CompanyEntity> {
        // Check if company already exists by matching name
        let existing = graph.list_companies()?;
        let matched = existing.into_iter().find(|c| {
            c.name.to_lowercase() == research.company_name.to_lowercase()
        });

        let entity = research.to_entity();

        match matched {
            Some(existing) => {
                // Update existing
                let updated = CompanyEntity {
                    id: existing.id,
                    name: entity.name,
                    industry: entity.industry.or(existing.industry),
                    description: entity.description.or(existing.description),
                    culture: entity.culture.or(existing.culture),
                    tech_stack: entity.tech_stack.or(existing.tech_stack),
                    strategic_angle: entity.strategic_angle.or(existing.strategic_angle),
                };
                graph.update_company(&updated)?;
                Ok(updated)
            }
            None => {
                // Create new
                let created = graph.create_company(
                    &entity.name,
                    entity.industry.as_deref(),
                    entity.description.as_deref(),
                    entity.culture.as_deref(),
                    entity.tech_stack.as_deref(),
                    entity.strategic_angle.as_deref(),
                )?;
                Ok(created)
            }
        }
    }

    /// Save company research to overview.md file
    pub fn save_to_file(&self, research: &CompanyResearch) -> Result<PathBuf> {
        let dir_name = research.company_name.to_lowercase().replace(' ', "-");
        let dir_path = self.knowledge_base_path.join("companies").join(&dir_name);
        fs::create_dir_all(&dir_path)?;

        let md_path = dir_path.join("overview.md");
        let content = research.to_markdown();
        fs::write(&md_path, content)?;

        Ok(md_path)
    }

    /// Get company research context for interview preparation
    pub fn get_interview_context(&self, company_name: &str) -> Result<String> {
        let dir_name = company_name.to_lowercase().replace(' ', "-");
        let md_path = self.knowledge_base_path
            .join("companies")
            .join(&dir_name)
            .join("overview.md");

        if !md_path.exists() {
            return Err(anyhow::anyhow!("No research found for company '{}'", company_name));
        }

        let research = CompanyResearch::from_markdown(&md_path)?;

        let mut ctx = format!(
            "## Company Research: {}\n\n",
            research.company_name
        );

        if let Some(v) = &research.industry {
            ctx.push_str(&format!("- **Industry**: {}\n", v));
        }
        if let Some(v) = &research.description {
            ctx.push_str(&format!("- **Description**: {}\n", v));
        }
        if let Some(v) = &research.culture {
            ctx.push_str(&format!("- **Culture**: {}\n", v));
        }
        if let Some(v) = &research.tech_stack {
            ctx.push_str(&format!("- **Tech Stack**: {}\n", v));
        }
        if let Some(v) = &research.strategic_angle {
            ctx.push_str(&format!("- **Strategic Angle**: {}\n", v));
        }

        if !research.key_challenges.is_empty() {
            ctx.push_str("\n**Key Challenges:**\n");
            for c in &research.key_challenges {
                ctx.push_str(&format!("- {}\n", c));
            }
        }

        if !research.interview_tips.is_empty() {
            ctx.push_str("\n**Interview Tips:**\n");
            for t in &research.interview_tips {
                ctx.push_str(&format!("- {}\n", t));
            }
        }

        Ok(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_existing_overview() {
        let md = "# CloudSEK Research\n\
            - **Industry**: Cybersecurity, AI-driven Threat Intelligence\n\
            - **Description**: Digital Risk Protection (DRP) platform\n\
            - **Culture**: Fast-paced, innovation-centric\n\
            - **Tech Stack**: Python, Go, ML, Elasticsearch\n\
            - **Strategic Angle**: Outside-in security approach\n\
            \n\
            ## Key Challenges\n\
            - Processing massive amounts of unstructured data\n\
            - False positive reduction in AI models\n";

        let dir = TempDir::new().unwrap();
        let md_path = dir.path().join("overview.md");
        fs::write(&md_path, &md).unwrap();

        let research = CompanyResearch::from_markdown(&md_path).unwrap();
        assert_eq!(research.company_name, "CloudSEK");
        assert_eq!(research.industry.unwrap(), "Cybersecurity, AI-driven Threat Intelligence");
        assert_eq!(research.key_challenges.len(), 2);
    }

    #[test]
    fn test_roundtrip() {
        let research = CompanyResearch {
            company_name: "TestCorp".into(),
            industry: Some("Tech".into()),
            description: Some("A test company".into()),
            culture: Some("Agile".into()),
            tech_stack: Some("Rust, Go".into()),
            strategic_angle: Some("Test angle".into()),
            key_challenges: vec!["Scaling".into()],
            products: vec!["Product A".into()],
            funding_rounds: vec![FundingRound {
                round: "Series A".into(),
                amount: Some("$10M".into()),
                date: Some("2024".into()),
                investors: Some("Sequoia".into()),
            }],
            competitors: vec!["Competitor X".into()],
            interview_tips: vec!["Emphasize scalability".into()],
            recent_news: vec!["Launched v2".into()],
        };

        let md = research.to_markdown();
        let dir = TempDir::new().unwrap();
        let md_path = dir.path().join("overview.md");
        fs::write(&md_path, &md).unwrap();

        let parsed = CompanyResearch::from_markdown(&md_path).unwrap();
        assert_eq!(parsed.company_name, "TestCorp");
        assert_eq!(parsed.industry.unwrap(), "Tech");
        assert_eq!(parsed.key_challenges[0], "Scaling");
        assert_eq!(parsed.funding_rounds[0].round, "Series A");
    }
}
