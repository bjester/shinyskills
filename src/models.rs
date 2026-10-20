// Copyright 2026 Blaine Jester
// SPDX-License-Identifier: MIT
//
// Derived from agentskills/skills-ref (Apache-2.0)
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// Validation modes that allow different sets of frontmatter fields.
///
/// Extensible for future modes like `Claude`, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationMode {
    /// Default validation - strict set of allowed fields
    Default,
    /// Vibe-specific validation - allows additional fields like `user-invocable`
    Vibe,
    // Future: Claude, Custom, etc.
}

impl ValidationMode {
    /// Get the allowed fields for this validation mode.
    pub fn allowed_fields(&self) -> &'static [&'static str] {
        match self {
            ValidationMode::Default => DEFAULT_ALLOWED_FIELDS,
            ValidationMode::Vibe => VIBE_ALLOWED_FIELDS,
        }
    }
}

const DEFAULT_ALLOWED_FIELDS: &[&str] = &[
    "name",
    "description",
    "license",
    "allowed-tools",
    "metadata",
    "compatibility",
];

const VIBE_ALLOWED_FIELDS: &[&str] = &[
    "name",
    "description",
    "license",
    "allowed-tools",
    "metadata",
    "compatibility",
    "user-invocable",
];

/// Fully resolved validation settings for one validation run.
///
/// Combines the built-in rules of a [`ValidationMode`] with the values read
/// from a repository configuration file. All state is immutable so the same
/// options can be reused across every skill of a repository.
#[derive(Debug, Clone)]
pub struct ValidationOptions {
    allowed_fields: HashSet<String>,
    ignore_files: globset::GlobSet,
    scan_extensions: Vec<String>,
}

impl ValidationOptions {
    /// Build options from already-resolved components.
    ///
    /// Crate-internal by design: callers go through the configuration resolver
    /// so glob patterns and extensions are normalized in one place.
    pub(crate) fn new(
        allowed_fields: HashSet<String>,
        ignore_files: globset::GlobSet,
        scan_extensions: Vec<String>,
    ) -> Self {
        Self {
            allowed_fields,
            ignore_files,
            scan_extensions,
        }
    }

    /// Whether `path` matches one of the configured `ignore_files` patterns.
    pub fn is_ignored(&self, path: &Path) -> bool {
        !self.ignore_files.is_empty() && self.ignore_files.is_match(path)
    }

    /// Whether the contents of `path` may be scanned for reference syntax.
    ///
    /// Only files whose extension is listed in `scan_extensions` are scanned.
    /// Extensions are matched case-insensitively.
    pub fn is_scannable(&self, path: &Path) -> bool {
        match extension_of(path) {
            Some(ext) => self.scan_extensions.iter().any(|e| e == &ext),
            None => false,
        }
    }

    /// Whether a frontmatter key is accepted by these options.
    pub fn allows_field(&self, field: &str) -> bool {
        self.allowed_fields.contains(field)
    }
}

/// Lowercased extension of `path` including the leading dot.
fn extension_of(path: &Path) -> Option<String> {
    let ext = path.extension().and_then(|e| e.to_str())?;
    if ext.is_empty() {
        return None;
    }
    Some(format!(".{}", ext.to_lowercase()))
}

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
