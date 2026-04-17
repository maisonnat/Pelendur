use crate::config::Config;
use crate::llm::ChatMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PracticeQuestion {
    pub question_type: String,
    pub question: String,
    pub tips: String,
    pub expected_aspects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerFeedback {
    pub structure_score: f64,
    pub specificity_score: f64,
    pub relevance_score: f64,
    pub overall_score: f64,
    pub feedback: String,
    pub improvements: Vec<String>,
    pub strong_points: Vec<String>,
}

pub struct PracticeEngine;

impl PracticeEngine {
    pub async fn generate_questions(
        mode: &str,
        profile_summary: &str,
        company_name: Option<&str>,
        config: &Config,
    ) -> Result<Vec<PracticeQuestion>, Box<dyn std::error::Error>> {
        let mode_desc = match mode {
            "behavioral" => "behavioral interview questions (Tell me about a time...)",
            "technical" => "technical interview questions related to the profile",
            "company" => "interview questions tailored to a specific company",
            _ => "general interview questions",
        };

        let prompt = format!(
            r#"Generate 3 {} for someone with this professional profile:

PROFILE:
{profile_summary}

{}

Return a JSON array with this structure (3 questions):
[
  {{
    "question_type": "{mode}",
    "question": "Full question text",
    "tips": "1-2 sentences of advice for answering well",
    "expected_aspects": ["aspect1", "aspect2"]
  }}
]

Return ONLY valid JSON array, no markdown fencing."#,
            mode_desc,
            if let Some(c) = company_name {
                format!("COMPANY FOCUS: Focus questions on {} company culture and values.", c)
            } else {
                String::new()
            }
        );

        let messages = vec![ChatMessage { role: "user".into(), content: prompt }];
        let response = crate::llm::generate_response_with_options(config, &messages, 800)
            .await
            .map_err(|e| format!("LLM failed: {}", e))?;

        let cleaned = response.trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let questions: Vec<PracticeQuestion> = serde_json::from_str(cleaned)
            .map_err(|e| format!("Failed to parse questions: {} — {}", e, &cleaned[..cleaned.len().min(200)]))?;
        Ok(questions)
    }

    pub async fn analyze_answer(
        question: &str,
        answer: &str,
        mode: &str,
        config: &Config,
    ) -> Result<AnswerFeedback, Box<dyn std::error::Error>> {
        let prompt = format!(
            r#"Analyze this interview answer:

QUESTION: {question}
ANSWER: {answer}
MODE: {mode}

Score and provide feedback:

Return JSON:
{{
  "structure_score": 0.0-1.0 (Does it follow STAR/clear structure?),
  "specificity_score": 0.0-1.0 (Does it include metrics, results, concrete details?),
  "relevance_score": 0.0-1.0 (Does it actually answer the question?),
  "overall_score": 0.0-1.0 (Weighted average),
  "feedback": "2-3 sentences of constructive feedback",
  "improvements": ["specific improvement 1", "specific improvement 2"],
  "strong_points": ["what was done well 1", "what was done well 2"]
}}

Return ONLY valid JSON, no markdown fencing."#
        );

        let messages = vec![ChatMessage { role: "user".into(), content: prompt }];
        let response = crate::llm::generate_response_with_options(config, &messages, 1000)
            .await
            .map_err(|e| format!("LLM failed: {}", e))?;

        let cleaned = response.trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let feedback: AnswerFeedback = serde_json::from_str(cleaned)
            .map_err(|e| format!("Failed to parse feedback: {} — {}", e, &cleaned[..cleaned.len().min(200)]))?;
        Ok(feedback)
    }
}