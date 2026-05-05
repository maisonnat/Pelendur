//! NotebookLM-powered company research loader.
//!
//! Uses the `notebooklm-mcp` CLI binary to run deep research on companies,
//! parses results into `CompanyResearch` structs, saves to
//! `knowledge/companies/<empresa>/overview.md`, and syncs to the knowledge graph.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────┐    ┌─────────────────┐    ┌──────────────────┐
//! │ CompanyResearcher│───>│ notebooklm-mcp  │───>│ Deep Research    │
//! │ (rust)           │    │ (subprocess)    │    │ (NotebookLM)     │
//! └──────────────────┘    └─────────────────┘    └──────────────────┘
//!         │                                              │
//!         v                                              v
//! ┌──────────────────┐    ┌─────────────────┐    ┌──────────────────┐
//! │ overview.md      │<───│ LLM extract     │<───│ Research Report  │
//! │ (knowledge/...)  │    │ (structured     │    │ (raw markdown)   │
//! └──────────────────┘    │  → CompanyRes.) │    └──────────────────┘
//!         │               └─────────────────┘
//!         v
//! ┌──────────────────┐
//! │ Knowledge Graph  │
//! │ (SQLite)         │
//! └──────────────────┘
//! ```

use crate::config::Config;
use crate::knowledge::company::{CompanyLoader, CompanyResearch};
use crate::knowledge::graph::KnowledgeGraph;
use crate::llm::ChatMessage;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

// ─── Constants ──────────────────────────────────────────────────────────

/// Name of the dedicated NotebookLM notebook used for company research.
const RESEARCH_NOTEBOOK_NAME: &str = "Pelendur — Company Research";

/// How long to wait for `notebooklm-mcp` CLI commands (in seconds).
const CLI_TIMEOUT_SECS: u64 = 60;

/// How long to wait for deep research to complete (in seconds).
const RESEARCH_TIMEOUT_SECS: u64 = 1500; // 25 minutes

/// Path to the notebooklm-mcp binary.
const NOTEBOOKLM_BINARY: &str = "/home/maiso/.local/bin/notebooklm-mcp";

// ─── Structs ─────────────────────────────────────────────────────────────

/// Configuration for the company researcher.
#[derive(Debug, Clone)]
pub struct CompanyResearchConfig {
    /// Path to the `notebooklm-mcp` binary.
    pub notebooklm_binary: PathBuf,
    /// Path to the `knowledge/` base directory.
    pub knowledge_base_path: PathBuf,
    /// Override notebook ID (optional). If set, uses this notebook instead
    /// of auto-discovering or creating one.
    pub research_notebook_id: Option<String>,
}

impl Default for CompanyResearchConfig {
    fn default() -> Self {
        Self {
            notebooklm_binary: PathBuf::from(NOTEBOOKLM_BINARY),
            knowledge_base_path: PathBuf::from("knowledge"),
            research_notebook_id: None,
        }
    }
}

/// Status returned after a research operation.
#[derive(Debug, Clone)]
pub struct ResearchStatus {
    pub company_name: String,
    pub overview_path: PathBuf,
    pub has_notebooklm: bool,
    pub research_done: bool,
    pub message: String,
}

/// The company researcher — orchestrates NotebookLM deep research + parsing.
pub struct CompanyResearcher {
    config: CompanyResearchConfig,
    // Cache the research notebook ID once discovered/created
    notebook_id: Arc<Mutex<Option<String>>>,
}

impl CompanyResearcher {
    /// Create a new researcher with the given config.
    pub fn new(config: CompanyResearchConfig) -> Self {
        Self {
            config,
            notebook_id: Arc::new(Mutex::new(None)),
        }
    }

    /// Create with default config.
    pub fn default_with_path(knowledge_base_path: &str) -> Self {
        Self::new(CompanyResearchConfig {
            knowledge_base_path: PathBuf::from(knowledge_base_path),
            ..Default::default()
        })
    }

    // ── High-level API ────────────────────────────────────────────────

    /// Run full research cycle for a company: NotebookLM deep research →
    /// LLM extraction → save to disk → sync to graph.
    ///
    /// Returns a `ResearchStatus` describing what happened.
    pub async fn research_company(
        &self,
        company_name: &str,
        graph: Option<&KnowledgeGraph>,
    ) -> Result<ResearchStatus> {
        // Check if research already exists
        let slug = company_name.to_lowercase().replace(' ', "-");
        let overview_path = self
            .config
            .knowledge_base_path
            .join("companies")
            .join(&slug)
            .join("overview.md");

        if overview_path.exists() {
            info!(
                "Company research already exists for '{}' at {:?}",
                company_name, overview_path
            );
            return Ok(ResearchStatus {
                company_name: company_name.to_string(),
                overview_path,
                has_notebooklm: false,
                research_done: false,
                message: format!("Research already exists for '{}'", company_name),
            });
        }

        // Step 1: Check if notebooklm-mcp is available
        let binary_exists = self.config.notebooklm_binary.exists();
        if !binary_exists {
            warn!(
                "notebooklm-mcp not found at {:?} — skipping deep research",
                self.config.notebooklm_binary
            );
            return self.fallback_research(company_name, graph).await;
        }

        // Step 2: Get or create the research notebook
        let notebook_id = self.get_or_create_notebook().await?;

        // Step 3: Run deep research
        info!(
            "Running deep research for '{}' (notebook: {})",
            company_name, notebook_id
        );
        let research_output = self.run_deep_research(&notebook_id, company_name).await;

        let research = match research_output {
            Ok(report) if !report.trim().is_empty() => {
                info!(
                    "Deep research completed for '{}' ({} chars)",
                    company_name,
                    report.len()
                );

                // Step 4: Extract structured data using LLM
                self.extract_research(company_name, &report).await?
            }
            Ok(_) => {
                warn!(
                    "Deep research returned empty result for '{}' — falling back to LLM-only",
                    company_name
                );
                return self.fallback_research(company_name, graph).await;
            }
            Err(e) => {
                warn!(
                    "Deep research failed for '{}': {} — falling back to LLM-only",
                    company_name, e
                );
                return self.fallback_research(company_name, graph).await;
            }
        };

        // Step 5: Save to file
        let loader = CompanyLoader::new(
            self.config
                .knowledge_base_path
                .to_str()
                .unwrap_or("knowledge"),
        );
        let path = loader.save_to_file(&research)?;

        // Step 6: Sync to graph
        if let Some(g) = graph {
            if let Err(e) = loader.sync_to_graph(&research, g) {
                warn!("Failed to sync company '{}' to graph: {}", company_name, e);
            } else {
                info!("Synced '{}' to knowledge graph", company_name);
            }
        }

        info!(
            "Company research saved for '{}' at {:?}",
            company_name, path
        );
        self.ask_followup_questions(&notebook_id, company_name, &research)
            .await;

        Ok(ResearchStatus {
            company_name: company_name.to_string(),
            overview_path: path,
            has_notebooklm: true,
            research_done: true,
            message: format!("Deep research completed for '{}'", company_name),
        })
    }

    /// Run research for ALL companies in the knowledge base that don't
    /// have research yet (based on the TEMPLATE or company names known
    /// to the graph).
    pub async fn research_all_missing(
        &self,
        graph: Option<&KnowledgeGraph>,
    ) -> Result<Vec<ResearchStatus>> {
        let mut results = Vec::new();
        let companies_dir = self.config.knowledge_base_path.join("companies");

        if !companies_dir.exists() {
            return Ok(results);
        }

        let mut entries = tokio::fs::read_dir(&companies_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let dir_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Skip template directory
            if dir_name == "TEMPLATE" || dir_name == "TEMPLATE.md" || dir_name.starts_with('.') {
                continue;
            }

            let overview_path = path.join("overview.md");
            if overview_path.exists() {
                let content = tokio::fs::read_to_string(&overview_path).await?;
                if !content.contains("> Auto-generated") && content.trim().len() > 100 {
                    continue; // Has real research data
                }
            }

            // Convert dir name back to company name (reversing slug)
            let company_name = dir_name.replace('-', " ");

            match self.research_company(&company_name, graph).await {
                Ok(status) => results.push(status),
                Err(e) => {
                    warn!("Failed to research '{}': {}", company_name, e);
                    results.push(ResearchStatus {
                        company_name,
                        overview_path: path.join("overview.md"),
                        has_notebooklm: false,
                        research_done: false,
                        message: format!("Research failed: {}", e),
                    });
                }
            }
        }

        Ok(results)
    }

    /// List companies that need research (stubs with TEMPLATE content).
    pub async fn list_unresearched_companies(&self) -> Result<Vec<String>> {
        let mut missing = Vec::new();
        let companies_dir = self.config.knowledge_base_path.join("companies");

        if !companies_dir.exists() {
            return Ok(missing);
        }

        let mut entries = tokio::fs::read_dir(&companies_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let dir_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            if dir_name == "TEMPLATE" || dir_name.starts_with('.') {
                continue;
            }

            let overview_path = path.join("overview.md");
            let needs_research = if overview_path.exists() {
                let content = tokio::fs::read_to_string(&overview_path).await?;
                content.contains("{{COMPANY_NAME}}") || content.trim().len() < 100
            } else {
                true
            };

            if needs_research {
                missing.push(dir_name.replace('-', " "));
            }
        }

        Ok(missing)
    }

    // ── NotebookLM Interaction ─────────────────────────────────────────

    /// Get or create the research notebook. Caches the ID.
    async fn get_or_create_notebook(&self) -> Result<String> {
        // Check cache first
        {
            let cached = self.notebook_id.lock().await;
            if let Some(id) = cached.as_ref() {
                return Ok(id.clone());
            }
        }

        // If an override was provided, use it
        if let Some(ref id) = self.config.research_notebook_id {
            let mut cached = self.notebook_id.lock().await;
            *cached = Some(id.clone());
            return Ok(id.clone());
        }

        // List notebooks via notebooklm-mcp
        let output = self
            .run_notebooklm_cli(&["list"])
            .await
            .context("Failed to list NotebookLM notebooks")?;

        // Parse output for our research notebook
        if let Some(id) = self.find_research_notebook(&output) {
            info!("Found existing research notebook: {}", id);
            let mut cached = self.notebook_id.lock().await;
            *cached = Some(id.clone());
            return Ok(id);
        }

        // For now, let the user create the notebook manually (avoid complex
        // notebook creation RPC). The research notebook creation RPC (CCqFvf)
        // isn't exposed as CLI.
        Err(anyhow::anyhow!(
            "Research notebook not found. Create a notebook called \
             '{}' in NotebookLM and make sure it's visible via \
             `notebooklm-mcp list`.",
            RESEARCH_NOTEBOOK_NAME
        ))
    }

    /// Parse notebook list output to find our research notebook.
    fn find_research_notebook(&self, output: &str) -> Option<String> {
        for line in output.lines() {
            // notebooklm-mcp list output format: "📓 UUID | Title (N sources)"
            if line.contains(RESEARCH_NOTEBOOK_NAME) {
                // Extract UUID — first word after the icon
                if let Some(id_part) = line.split('|').next() {
                    let uuid = id_part
                        .split_whitespace()
                        .find(|w| w.len() > 30 && w.contains('-'))
                        .or_else(|| {
                            // Try finding UUID directly
                            id_part
                                .split(|c: char| !c.is_alphanumeric() && c != '-')
                                .find(|w| w.len() == 36 && w.contains('-'))
                        });
                    return uuid.map(|s| s.trim().to_string());
                }
            }
        }
        None
    }

    /// Run deep research on a company using the NotebookLM CLI.
    async fn run_deep_research(&self, notebook_id: &str, company_name: &str) -> Result<String> {
        let query = format!(
            "Research company {company_name}. I need comprehensive data for interview preparation. \
             Find the following about {company_name}: \
             1. Industry sector and sub-sector \
             2. Company description - what do they do? \
             3. Company culture and values \
             4. Tech stack and tools they use \
             5. Strategic angle or competitive advantage \
             6. Key business and technical challenges they face \
             7. Products and services they offer \
             8. Main competitors \
             9. Funding rounds (Series, Amount, Date, Investors) \
             10. Recent news and developments (last 6 months) \
             11. Interview tips - what do they look for in candidates? \
             12. Engineering culture and interview process",
            company_name = company_name
        );

        info!(
            "Starting deep research for '{}' (CLI research command)",
            company_name
        );

        let output = self
            .run_notebooklm_cli_with_timeout(
                &["research", "--notebook-id", notebook_id, "--query", &query],
                RESEARCH_TIMEOUT_SECS,
            )
            .await
            .context(format!(
                "notebooklm-mcp research failed for '{}'",
                company_name
            ))?;

        Ok(output)
    }

    /// Ask follow-up questions on the notebook to enrich structured data.
    async fn ask_followup_questions(
        &self,
        notebook_id: &str,
        company_name: &str,
        research: &CompanyResearch,
    ) {
        // Ask for tech stack details if missing
        if research.tech_stack.is_none() {
            if let Ok(answer) = self
                .ask_notebook(
                    notebook_id,
                    &format!(
                        "What is the technology stack used by {}? \
                         List programming languages, frameworks, databases, infrastructure tools.",
                        company_name
                    ),
                )
                .await
            {
                info!(
                    "Follow-up tech stack question answered for '{}' ({} chars)",
                    company_name,
                    answer.len()
                );
            }
        }
    }

    /// Ask a question to the research notebook.
    async fn ask_notebook(&self, notebook_id: &str, question: &str) -> Result<String> {
        self.run_notebooklm_cli(&["ask", "--notebook-id", notebook_id, "--question", question])
            .await
    }

    /// Run a notebooklm-mcp CLI command and return stdout.
    async fn run_notebooklm_cli(&self, args: &[&str]) -> Result<String> {
        self.run_notebooklm_cli_with_timeout(args, CLI_TIMEOUT_SECS)
            .await
    }

    /// Run a notebooklm-mcp CLI command with custom timeout.
    async fn run_notebooklm_cli_with_timeout(
        &self,
        args: &[&str],
        timeout_secs: u64,
    ) -> Result<String> {
        let binary = &self.config.notebooklm_binary;

        info!("Running: {} {}", binary.display(), args.join(" "));

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            tokio::process::Command::new(binary).args(args).output(),
        )
        .await
        .context("notebooklm-mcp command timed out")?
        .context("Failed to execute notebooklm-mcp command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            error!(
                "notebooklm-mcp failed (exit: {}):\nstdout:\n{}\nstderr:\n{}",
                output.status, stdout, stderr
            );
            anyhow::bail!(
                "notebooklm-mcp exited with {}: {}",
                output.status,
                stderr.lines().next().unwrap_or("unknown error")
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    }

    // ── LLM Extraction ─────────────────────────────────────────────────

    /// Extract structured `CompanyResearch` from raw NotebookLM research output
    /// using the local LLM (Ollama).
    async fn extract_research(
        &self,
        company_name: &str,
        research_report: &str,
    ) -> Result<CompanyResearch> {
        info!("Extracting structured data for '{}' via LLM", company_name);

        // Truncate if too long for LLM context
        let truncated = if research_report.len() > 12000 {
            warn!(
                "Research report is {} chars, truncating to first 12000",
                research_report.len()
            );
            &research_report[..12000]
        } else {
            research_report
        };

        // Build the extraction prompt
        let prompt = format!(
            r#"Extract structured company information from the research report below.

Company name: {company_name}

Research Report:
```
{report}
```

Return ONLY a valid JSON object with these fields (use null for missing values):
- "company_name": string
- "industry": string or null
- "description": string or null
- "culture": string or null
- "tech_stack": string or null
- "strategic_angle": string or null
- "key_challenges": array of strings
- "products": array of strings
- "competitors": array of strings
- "funding_rounds": array of {{ "round": string, "amount": string or null, "date": string or null, "investors": string or null }}
- "interview_tips": array of strings
- "recent_news": array of strings

Rules:
- Be concise but informative
- Use the research report as primary source
- For missing information, leave as null or empty array
- Do NOT include any text outside the JSON object
"#,
            company_name = company_name,
            report = truncated
        );

        // We need a Config to call the LLM. Create a minimal one from env.
        let config = Config::from_env()?;

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are an expert data extraction assistant. Extract structured company information from research reports. Output ONLY valid JSON.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        let response = crate::llm::generate_response_with_options(&config, &messages, 2000)
            .await
            .context("LLM extraction failed")?;

        // Parse JSON from response
        let json_str = extract_json(&response)
            .ok_or_else(|| anyhow::anyhow!("No JSON found in LLM response: {}", response))?;

        let research: CompanyResearch = serde_json::from_str(&json_str).context(format!(
            "Failed to parse LLM output as CompanyResearch: {}",
            json_str
        ))?;

        info!(
            "Extracted structured data for '{}': industry={:?}, challenges={}, products={}, tips={}",
            research.company_name,
            research.industry,
            research.key_challenges.len(),
            research.products.len(),
            research.interview_tips.len(),
        );

        Ok(research)
    }

    // ── Fallback ───────────────────────────────────────────────────────

    /// Fallback: use the LLM directly (without NotebookLM) to research a company
    /// from web knowledge. This is used when NotebookLM is unavailable.
    async fn fallback_research(
        &self,
        company_name: &str,
        graph: Option<&KnowledgeGraph>,
    ) -> Result<ResearchStatus> {
        info!("Using LLM-only fallback for company '{}'", company_name);

        let config = Config::from_env()?;
        let prompt = format!(
            r#"Generate comprehensive company research for interview preparation.

Company: {company_name}

Return ONLY a valid JSON object with these fields (use null for missing values):
- "company_name": "{company_name}"
- "industry": string
- "description": string
- "culture": string
- "tech_stack": string
- "strategic_angle": string
- "key_challenges": array of strings
- "products": array of strings
- "competitors": array of strings
- "funding_rounds": array of {{ "round": string, "amount": string or null, "date": string or null, "investors": string or null }}
- "interview_tips": array of strings
- "recent_news": array of strings

Rules:
- Be accurate and specific
- If you're uncertain about a field, use your best knowledge
- For funding: include known rounds or leave empty
- For interview tips: suggest what this type of company typically looks for
- Output ONLY the JSON object, no other text
"#,
            company_name = company_name,
        );

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are an expert company research assistant for interview preparation. Generate accurate, structured company information based on your training data.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        let response = crate::llm::generate_response_with_options(&config, &messages, 2000)
            .await
            .context("LLM fallback research failed")?;

        let json_str = extract_json(&response)
            .ok_or_else(|| anyhow::anyhow!("No JSON in LLM fallback response"))?;

        let research: CompanyResearch = serde_json::from_str(&json_str)
            .context(format!("Failed to parse fallback LLM output: {}", json_str))?;

        let loader = CompanyLoader::new(
            self.config
                .knowledge_base_path
                .to_str()
                .unwrap_or("knowledge"),
        );
        let path = loader.save_to_file(&research)?;

        if let Some(g) = graph {
            if let Err(e) = loader.sync_to_graph(&research, g) {
                warn!("Failed to sync company to graph: {}", e);
            }
        }

        info!(
            "LLM-only research saved for '{}' at {:?}",
            company_name, path
        );

        Ok(ResearchStatus {
            company_name: company_name.to_string(),
            overview_path: path,
            has_notebooklm: false,
            research_done: true,
            message: format!("LLM-based research completed for '{}'", company_name),
        })
    }
}

// ─── Utilities ──────────────────────────────────────────────────────────

/// Extract a JSON object from text that may have surrounding markdown/code fences.
fn extract_json(text: &str) -> Option<String> {
    let text = text.trim();

    // Try to find JSON inside ```json ... ``` fences first
    if let Some(start) = text.find("```json") {
        let after = &text[start + 7..];
        if let Some(end) = after.find("```") {
            let json = after[..end].trim();
            if json.starts_with('{') {
                return Some(json.to_string());
            }
        }
    }

    // Try ``` ... ``` (any code block)
    if let Some(start) = text.find("```") {
        let after = &text[start + 3..];
        if let Some(end) = after.find("```") {
            let json = after[..end].trim();
            if json.starts_with('{') {
                return Some(json.to_string());
            }
        }
    }

    // Try to find a JSON object directly (starts with {)
    if let Some(start) = text.find('{') {
        let mut depth = 0;
        let mut end = None;
        for (i, ch) in text[start..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(start + i + 1);
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(end) = end {
            return Some(text[start..end].to_string());
        }
    }

    None
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::company::{CompanyResearch, FundingRound};

    #[test]
    fn test_extract_json_from_code_fence() {
        let input = r#"
Here is the result:
```json
{"company_name": "TestCorp", "industry": "Tech"}
```
Some trailing text."#;
        let result = extract_json(input).unwrap();
        assert!(result.contains("\"company_name\": \"TestCorp\""));
    }

    #[test]
    fn test_extract_json_bare() {
        let input = r#"{"company_name": "TestCorp", "industry": "Tech"}"#;
        let result = extract_json(input).unwrap();
        assert!(result.contains("\"company_name\""));
    }

    #[test]
    fn test_extract_json_without_fences() {
        let input = r#"Some text before {"company_name": "Acme"} and after"#;
        let result = extract_json(input).unwrap();
        assert_eq!(result, r#"{"company_name": "Acme"}"#);
    }

    #[test]
    fn test_extract_json_no_json() {
        let input = "This is just plain text without any JSON";
        let result = extract_json(input);
        assert!(result.is_none());
    }

    #[test]
    fn test_research_status_creation() {
        let status = ResearchStatus {
            company_name: "TestCo".to_string(),
            overview_path: PathBuf::from("knowledge/companies/testco/overview.md"),
            has_notebooklm: true,
            research_done: true,
            message: "All good".to_string(),
        };
        assert_eq!(status.company_name, "TestCo");
        assert!(status.research_done);
    }

    #[test]
    fn test_company_research_struct_default() {
        let config = CompanyResearchConfig::default();
        assert_eq!(config.notebooklm_binary, PathBuf::from(NOTEBOOKLM_BINARY));
        assert_eq!(config.knowledge_base_path, PathBuf::from("knowledge"));
        assert!(config.research_notebook_id.is_none());
    }

    #[test]
    fn test_company_research_slug_convention() {
        let name = "CloudSEK Inc";
        let slug = name.to_lowercase().replace(' ', "-");
        // The module saves to knowledge/companies/<slug>/overview.md
        assert_eq!(slug, "cloudsek-inc");
    }
}
