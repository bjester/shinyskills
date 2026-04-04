// Copyright 2026 Blaine Jester
// SPDX-License-Identifier: MIT
//
// Derived from agentskills/skills-ref (Apache-2.0)
use serde::{Deserialize, Serialize};

/// Properties parsed from a skill's SKILL.md frontmatter.
///
/// This struct represents the metadata defined in the YAML frontmatter
/// of a SKILL.md file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillProperties {
    /// Skill name in kebab-case (required).
    pub name: String,

    /// What the skill does and when the model should use it (required).
    pub description: String,

    /// License for the skill (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Compatibility information for the skill (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<String>,

    /// Tool patterns the skill requires (optional, experimental).
    #[serde(rename = "allowed-tools", skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<String>,

    /// Key-value pairs for client-specific properties (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

impl SkillProperties {
    /// Convert to a dictionary, excluding None values.
    ///
    /// This produces a HashMap suitable for JSON serialization.
    pub fn to_dict(&self) -> std::collections::HashMap<String, serde_json::Value> {
        let mut result = std::collections::HashMap::new();
        result.insert(
            "name".to_string(),
            serde_json::Value::String(self.name.clone()),
        );
        result.insert(
            "description".to_string(),
            serde_json::Value::String(self.description.clone()),
        );

        if let Some(ref license) = self.license {
            result.insert(
                "license".to_string(),
                serde_json::Value::String(license.clone()),
            );
        }
        if let Some(ref compatibility) = self.compatibility {
            result.insert(
                "compatibility".to_string(),
                serde_json::Value::String(compatibility.clone()),
            );
        }
        if let Some(ref allowed_tools) = self.allowed_tools {
            result.insert(
                "allowed-tools".to_string(),
                serde_json::Value::String(allowed_tools.clone()),
            );
        }
        if let Some(ref metadata) = self.metadata {
            let metadata_json: std::collections::HashMap<String, serde_json::Value> = metadata
                .iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();
            result.insert(
                "metadata".to_string(),
                serde_json::Value::Object(metadata_json.into_iter().collect()),
            );
        }

        result
    }
}
