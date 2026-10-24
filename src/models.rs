// Copyright 2026 Blaine Jester
// SPDX-License-Identifier: MIT
//
// Derived from agentskills/skills-ref (Apache-2.0)
//! Data models and resolved options for skill validation.
use globset::GlobSet;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Validation modes that allow different sets of frontmatter fields.
///
/// Extensible for future modes like `Claude`, etc.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValidationMode {
    /// Default validation - strict set of allowed fields
    #[default]
    Default,
    /// Vibe-specific validation - allows additional fields like `user-invocable`
    Vibe,
    // Future: Claude, Custom, etc.
}

impl ValidationMode {
    /// Get the allowed fields for this validation mode.
    #[must_use]
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
    ignore_files: GlobSet,
    scan_extensions: Vec<String>,
}

impl Default for ValidationOptions {
    /// Options for the built-in rules of [`ValidationMode::Default`].
    ///
    /// Equivalent to resolving options with no configuration file present.
    fn default() -> Self {
        Self {
            allowed_fields: ValidationMode::Default
                .allowed_fields()
                .iter()
                .map(|field| (*field).to_string())
                .collect(),
            ignore_files: GlobSet::empty(),
            scan_extensions: vec![".md".to_string()],
        }
    }
}

impl ValidationOptions {
    /// Build options from already-resolved components.
    ///
    /// Crate-internal by design: callers go through the configuration resolver
    /// so glob patterns and extensions are normalized in one place.
    pub(crate) fn new(
        allowed_fields: HashSet<String>,
        ignore_files: GlobSet,
        scan_extensions: Vec<String>,
    ) -> Self {
        Self {
            allowed_fields,
            ignore_files,
            scan_extensions,
        }
    }

    /// Whether `path` matches one of the configured `ignore_files` patterns.
    #[must_use]
    pub fn is_ignored(&self, path: &Path) -> bool {
        !self.ignore_files.is_empty() && self.ignore_files.is_match(path)
    }

    /// Whether the contents of `path` may be scanned for reference syntax.
    ///
    /// Only files whose extension is listed in `scan_extensions` are scanned.
    /// Extensions are matched case-insensitively.
    #[must_use]
    pub fn is_scannable(&self, path: &Path) -> bool {
        match extension_of(path) {
            Some(extension) => self.scan_extensions.iter().any(|e| e == &extension),
            None => false,
        }
    }

    /// Whether a frontmatter key is accepted by these options.
    #[must_use]
    pub fn allows_field(&self, field: &str) -> bool {
        self.allowed_fields.contains(field)
    }

    /// Accepted frontmatter keys, sorted for stable diagnostics.
    pub(crate) fn allowed_field_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.allowed_fields.iter().cloned().collect();
        names.sort();
        names
    }
}

/// Lowercased extension of `path` including the leading dot.
fn extension_of(path: &Path) -> Option<String> {
    let extension = path.extension().and_then(|e| e.to_str())?;
    if extension.is_empty() {
        return None;
    }
    Some(format!(".{}", extension.to_lowercase()))
}

/// Metadata and markdown body parsed from a SKILL.md file.
///
/// The metadata holds every YAML frontmatter key, so callers decide which keys
/// matter. The body is the markdown that follows the closing `---` delimiter.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ParsedFrontmatter {
    /// Frontmatter keys mapped to their parsed YAML values.
    pub metadata: HashMap<String, Value>,

    /// Markdown body that follows the frontmatter block.
    pub body: String,
}

/// Outcome of a validation run.
///
/// `errors` make a skill invalid, `warnings` are advisory. A report with only
/// warnings still passes validation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
#[must_use]
pub struct ValidationReport {
    /// Advisory findings such as orphaned files.
    pub warnings: Vec<String>,

    /// Findings that make the skill invalid.
    pub errors: Vec<String>,
}

impl ValidationReport {
    /// Create a report from already-collected findings.
    pub fn new(warnings: Vec<String>, errors: Vec<String>) -> Self {
        Self { warnings, errors }
    }

    /// Create a report holding one error and no warnings.
    pub fn single_error(error: String) -> Self {
        Self {
            warnings: Vec::new(),
            errors: vec![error],
        }
    }

    /// Whether the validated skill may be used, ignoring warnings.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Properties parsed from a skill's SKILL.md frontmatter.
///
/// This struct represents the metadata defined in the YAML frontmatter
/// of a SKILL.md file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
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
    pub metadata: Option<HashMap<String, String>>,
}

impl SkillProperties {
    /// Convert to a map, excluding None values.
    ///
    /// This produces a HashMap suitable for JSON serialization.
    #[must_use]
    pub fn to_map(&self) -> HashMap<String, Value> {
        let mut result = HashMap::new();
        result.insert("name".to_string(), Value::String(self.name.clone()));
        result.insert(
            "description".to_string(),
            Value::String(self.description.clone()),
        );

        if let Some(license) = &self.license {
            result.insert("license".to_string(), Value::String(license.clone()));
        }
        if let Some(compatibility) = &self.compatibility {
            result.insert(
                "compatibility".to_string(),
                Value::String(compatibility.clone()),
            );
        }
        if let Some(allowed_tools) = &self.allowed_tools {
            result.insert(
                "allowed-tools".to_string(),
                Value::String(allowed_tools.clone()),
            );
        }
        if let Some(metadata) = &self.metadata {
            let mut object = Map::new();
            for (key, value) in metadata {
                object.insert(key.clone(), Value::String(value.clone()));
            }
            result.insert("metadata".to_string(), Value::Object(object));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_mode_defaults_to_default() {
        assert_eq!(ValidationMode::default(), ValidationMode::Default);
    }

    #[test]
    fn validation_options_defaults_match_default_mode() {
        let options = ValidationOptions::default();

        assert!(options.allows_field("name"));
        assert!(!options.allows_field("user-invocable"));
        assert!(!options.is_ignored(Path::new("CHANGELOG.md")));
        assert!(options.is_scannable(Path::new("SKILL.md")));
        assert!(!options.is_scannable(Path::new("scripts/run.sh")));
    }

    #[test]
    fn validation_report_tracks_validity() {
        let with_warnings = ValidationReport::new(vec!["w".to_string()], Vec::new());
        assert!(with_warnings.is_valid());
        assert_eq!(with_warnings.warnings, ["w"]);

        let with_error = ValidationReport::single_error("e".to_string());
        assert!(!with_error.is_valid());
        assert_eq!(with_error.errors, ["e"]);

        assert!(ValidationReport::default().is_valid());
    }

    #[test]
    fn skill_properties_to_map_excludes_none_and_keeps_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("owner".to_string(), "team".to_string());

        let properties = SkillProperties {
            name: "pdf-reader".to_string(),
            description: "Read text from PDF files".to_string(),
            license: Some("MIT".to_string()),
            compatibility: None,
            allowed_tools: None,
            metadata: Some(metadata),
        };

        let map = properties.to_map();

        assert_eq!(
            map.get("name"),
            Some(&Value::String("pdf-reader".to_string()))
        );
        assert_eq!(
            map.get("description"),
            Some(&Value::String("Read text from PDF files".to_string()))
        );
        assert_eq!(map.get("license"), Some(&Value::String("MIT".to_string())));
        assert!(!map.contains_key("compatibility"));
        assert!(!map.contains_key("allowed-tools"));

        let object: &Map<String, Value> = map
            .get("metadata")
            .and_then(Value::as_object)
            .expect("metadata is a JSON object");
        assert_eq!(
            object.get("owner"),
            Some(&Value::String("team".to_string()))
        );
    }
}
