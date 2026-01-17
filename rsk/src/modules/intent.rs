use serde::{Deserialize, Serialize};
use regex::Regex;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SkillPattern {
    Auditor,
    Computer,
    Transformer,
    Generic,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SkillComplexity {
    Low,
    Moderate,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredIntent {
    pub pattern: SkillPattern,
    pub complexity: SkillComplexity,
    pub rsk_modules: Vec<String>,
}

pub fn classify_intent(intent: &str) -> anyhow::Result<StructuredIntent> {
    let intent_lower = intent.to_lowercase();
    
    // Pattern detection
    let pattern = if intent_lower.contains("audit") || intent_lower.contains("validate") || intent_lower.contains("verify") || intent_lower.contains("check") {
        SkillPattern::Auditor
    } else if intent_lower.contains("calculate") || intent_lower.contains("compute") || intent_lower.contains("math") || intent_lower.contains("algorithm") {
        SkillPattern::Computer
    } else if intent_lower.contains("transform") || intent_lower.contains("convert") || intent_lower.contains("parse") {
        SkillPattern::Transformer
    } else {
        SkillPattern::Generic
    };

    // Complexity detection
    let complexity = if intent_lower.contains("complex") || intent_lower.contains("heavy") || intent_lower.contains("performance") {
        SkillComplexity::High
    } else if intent_lower.contains("simple") || intent_lower.contains("basic") {
        SkillComplexity::Low
    } else {
        SkillComplexity::Moderate
    };

    // RSK module suggestions
    let mut rsk_modules = Vec::new();
    match pattern {
        SkillPattern::Auditor => {
            rsk_modules.push("rule_compiler".to_string());
            rsk_modules.push("logic_validator".to_string());
        },
        SkillPattern::Computer => {
            rsk_modules.push("math".to_string());
            rsk_modules.push("graph".to_string());
        },
        SkillPattern::Transformer => {
            rsk_modules.push("text_processor".to_string());
            rsk_modules.push("compression".to_string());
        },
        SkillPattern::Generic => {
            rsk_modules.push("yaml_processor".to_string());
        }
    }

    Ok(StructuredIntent {
        pattern,
        complexity,
        rsk_modules,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_auditor_intent() {
        let intent = "Audit my code for security";
        let result = classify_intent(intent).unwrap();
        assert_eq!(result.pattern, SkillPattern::Auditor);
        assert!(result.rsk_modules.contains(&"rule_compiler".to_string()));
    }

    #[test]
    fn test_classify_computer_intent() {
        let intent = "Calculate high-performance mathematical derivatives";
        let result = classify_intent(intent).unwrap();
        assert_eq!(result.pattern, SkillPattern::Computer);
        assert_eq!(result.complexity, SkillComplexity::High);
    }

    #[test]
    fn test_classify_transformer_intent() {
        let intent = "Convert CSV data to JSON objects";
        let result = classify_intent(intent).unwrap();
        assert_eq!(result.pattern, SkillPattern::Transformer);
    }

    #[test]
    fn test_adversarial_intent() {
        // Test "jailbreak" where keywords conflict
        let intent = "Audit my math calculations";
        let result = classify_intent(intent).unwrap();
        // Currently prioritizes Audit (Auditor)
        assert_eq!(result.pattern, SkillPattern::Auditor);
    }
}
