// Skills knowledge module
// Provides skill-related knowledge management capabilities

use serde::{Deserialize, Serialize};

/// Represents a skill with proficiency level and usage context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub level: String,
    pub years: u8,
    pub projects: Vec<String>,
    pub context: Option<String>,
}

/// Provider for skill-related knowledge
pub trait SkillProvider {
    fn get_skills(&self) -> Vec<Skill>;
    fn find_related_skills(&self, query: &str) -> Vec<&Skill>;
}

impl Skill {
    pub fn new(name: &str, level: &str, years: u8) -> Self {
        Self {
            name: name.to_string(),
            level: level.to_string(),
            years,
            projects: Vec::new(),
            context: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_creation() {
        let skill = Skill::new("Rust", "expert", 5);
        assert_eq!(skill.name, "Rust");
        assert_eq!(skill.level, "expert");
        assert_eq!(skill.years, 5);
    }
}
