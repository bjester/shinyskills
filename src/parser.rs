// Copyright 2026 Blaine Jester
// SPDX-License-Identifier: MIT
//
// Derived from agentskills/skills-ref (Apache-2.0)
//! YAML frontmatter parsing, skill property extraction, and file reference scanning.
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde_json::Value;

use crate::errors::SkillError;
pub use crate::models::ParsedFrontmatter;
use crate::models::SkillProperties;

const EXCLUDED_REFERENCE_NAMES: &[&str] = &["venv", "__pycache__", "node_modules"];

thread_local! {
    static REGEX_CACHE: RefCell<HashMap<String, Regex>> = RefCell::new(HashMap::new());
}

/// Walk up directory tree to find if file is inside a skill directory.
///
/// Returns Some(skill_dir) if a SKILL.md is found in any ancestor directory
/// within the specified constraints.
///
/// # Arguments
/// * `file` - Path to a file that might be inside a skill directory
/// * `context_dir` - Optional directory to stop walking at (don't search above this)
/// * `max_depth` - Maximum number of levels to walk up (0 means only check file's directory)
///
/// # Returns
/// The skill directory path if found, None otherwise
#[must_use]
pub fn find_skill_dir_for_file(
    file: &Path,
    context_dir: Option<&Path>,
    max_depth: usize,
) -> Option<PathBuf> {
    // Start from the file's parent directory
    let mut current = file.parent()?;
    let mut depth = 0;

    // Check current directory first
    if find_skill_md(current).is_some() {
        return Some(current.to_path_buf());
    }

    // Walk up the tree
    while let Some(parent) = current.parent() {
        // Check max depth constraint
        if depth >= max_depth {
            break;
        }

        // Check context directory constraint
        if let Some(context) = context_dir {
            if !parent.starts_with(context) {
                break;
            }
        }

        if find_skill_md(parent).is_some() {
            return Some(parent.to_path_buf());
        }

        current = parent;
        depth += 1;
    }

    None
}

/// Find the SKILL.md file in a skill directory.
///
/// Prefers SKILL.md (uppercase) but accepts skill.md (lowercase).
///
/// # Arguments
/// * `skill_dir` - Path to the skill directory
///
/// # Returns
/// Path to the SKILL.md file, or None if not found
#[must_use]
pub fn find_skill_md(skill_dir: &Path) -> Option<PathBuf> {
    for name in &["SKILL.md", "skill.md"] {
        let path = skill_dir.join(name);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Parse YAML frontmatter from SKILL.md content.
///
/// Expects content to start with `---` followed by YAML, then `---`.
///
/// # Arguments
/// * `content` - Raw content of SKILL.md file
///
/// # Returns
/// [`ParsedFrontmatter`] holding the frontmatter map and the markdown body
///
/// # Errors
/// Returns `ParseError` if frontmatter is missing, malformed, or contains invalid YAML.
pub fn parse_frontmatter(content: &str) -> Result<ParsedFrontmatter, SkillError> {
    if !content.starts_with("---") {
        return Err(SkillError::ParseError(
            "SKILL.md must start with YAML frontmatter (---)".to_string(),
        ));
    }

    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return Err(SkillError::ParseError(
            "SKILL.md frontmatter not properly closed with ---".to_string(),
        ));
    }

    let frontmatter_text = parts[1];
    let body = parts[2].trim().to_string();

    let metadata: HashMap<String, Value> = serde_yaml::from_str(frontmatter_text)
        .map_err(|e| SkillError::ParseError(format!("Invalid YAML in frontmatter: {}", e)))?;

    Ok(ParsedFrontmatter { metadata, body })
}

/// Extract an optional string field from metadata.
fn extract_string_field(metadata: &HashMap<String, Value>, key: &str) -> Option<String> {
    match metadata.get(key) {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

/// Extract the metadata sub-mapping as a HashMap<String, String>.
fn extract_metadata_field(metadata: &HashMap<String, Value>) -> Option<HashMap<String, String>> {
    match metadata.get("metadata") {
        Some(Value::Object(object)) => {
            let mut map = HashMap::new();
            for (k, v) in object {
                let value = match v {
                    Value::String(s) => s.clone(),
                    _ => v.to_string(),
                };
                map.insert(k.clone(), value);
            }
            Some(map)
        }
        _ => None,
    }
}

/// Read skill properties from SKILL.md frontmatter.
///
/// This function parses the frontmatter and returns properties.
/// It does NOT perform full validation. Use `validate()` for that.
///
/// # Arguments
/// * `skill_dir` - Path to the skill directory
///
/// # Returns
/// `SkillProperties` with parsed metadata
///
/// # Errors
/// Returns `ParseError` if SKILL.md is missing or has invalid YAML.
/// Returns `ValidationError` if required fields (name, description) are missing.
pub fn read_properties(skill_dir: &Path) -> Result<SkillProperties, SkillError> {
    let skill_md = match find_skill_md(skill_dir) {
        Some(path) => path,
        None => {
            return Err(SkillError::ParseError(format!(
                "SKILL.md not found in {}",
                skill_dir.display()
            )));
        }
    };

    let content = fs::read_to_string(&skill_md)
        .map_err(|e| SkillError::ParseError(format!("Failed to read SKILL.md: {}", e)))?;

    let ParsedFrontmatter { metadata, .. } = parse_frontmatter(&content)?;

    let name = match metadata.get("name") {
        Some(Value::String(n)) => n.clone(),
        Some(_) => {
            return Err(SkillError::ValidationError(
                "Field 'name' must be a non-empty string".to_string(),
            ));
        }
        None => {
            return Err(SkillError::ValidationError(
                "Missing required field in frontmatter: name".to_string(),
            ));
        }
    };

    let description = match metadata.get("description") {
        Some(Value::String(d)) => d.clone(),
        Some(_) => {
            return Err(SkillError::ValidationError(
                "Field 'description' must be a non-empty string".to_string(),
            ));
        }
        None => {
            return Err(SkillError::ValidationError(
                "Missing required field in frontmatter: description".to_string(),
            ));
        }
    };

    Ok(SkillProperties {
        name,
        description,
        license: extract_string_field(&metadata, "license"),
        compatibility: extract_string_field(&metadata, "compatibility"),
        allowed_tools: extract_string_field(&metadata, "allowed-tools"),
        metadata: extract_metadata_field(&metadata),
    })
}

fn recurse_scan_skill_files(
    target_path: &Path,
    relative_to: &Path,
) -> Result<Vec<PathBuf>, io::Error> {
    let mut files = Vec::new();

    let mut entries: Vec<_> = fs::read_dir(target_path)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    entries.sort();

    for path in entries {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if file_name.starts_with('.') || EXCLUDED_REFERENCE_NAMES.contains(&file_name) {
            continue;
        }

        if path.is_symlink() {
            continue;
        }

        let relative_path = path.strip_prefix(relative_to).unwrap_or(&path);

        if path.is_dir() {
            let mut subdirectory_files = recurse_scan_skill_files(&path, relative_to)?;
            files.append(&mut subdirectory_files);
            continue;
        }

        if file_name.to_lowercase() == "skill.md" {
            continue;
        }

        files.push(relative_path.to_path_buf());
    }

    Ok(files)
}

/// Scan skill directory for all files (excluding SKILL.md).
///
/// Returns relative paths from skill root, e.g., ['references/GUIDE.md', 'scripts/run.py'].
///
/// Excludes:
/// - SKILL.md / skill.md (the main instruction file)
/// - Hidden files/directories (starting with .)
/// - Symlinks
/// - Directories named: venv, __pycache__, node_modules
///
/// # Arguments
/// * `skill_dir` - Path to the skill directory
///
/// # Returns
/// List of relative file paths (as PathBuf objects), sorted alphabetically
pub fn scan_skill_files(skill_dir: &Path) -> Result<Vec<PathBuf>, io::Error> {
    let files = recurse_scan_skill_files(skill_dir, skill_dir)?;
    Ok(files)
}

/// Status of a file reference check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ReferenceStatus {
    /// The file is referenced by its correct path.
    Found,
    /// The file exists but no skill file refers to it.
    Orphaned,
    /// Some reference uses a path that differs from the file's real location.
    BrokenPath,
}

/// Check if a file path is referenced in the SKILL.md body.
///
/// Searches for the path in multiple formats:
/// - Markdown links: [text](path/to/file)
/// - Inline code: `path/to/file`
/// - Bare text: path/to/file (as standalone text)
///
/// Normalizes Windows-style backslashes to forward slashes for comparison.
///
/// # Arguments
/// * `body` - Markdown body content from SKILL.md
/// * `relative_path` - Relative path to search for (e.g., Path('references/GUIDE.md'))
///
/// # Returns
/// `ReferenceStatus`:
/// - `Found`: All references to this file use the correct path
/// - `BrokenPath`: At least one reference uses a wrong path (e.g., docs/X.md vs references/X.md)
/// - `Orphaned`: File is not referenced at all
#[must_use]
pub fn is_path_referenced(body: &str, relative_path: &Path) -> ReferenceStatus {
    let path_str_posix = relative_path.to_string_lossy().replace('\\', "/");
    let path_str_basename = relative_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let normalized_body = body.replace('\\', "/");

    // Build pattern to match the basename with optional directory prefix
    let pattern = format!(
        r"(?:^|[^\w._/-])((?:[\w._-]*/)*{})(?:$|[^\w._/-])",
        regex::escape(path_str_basename)
    );

    // Use thread-local cache to avoid recompiling regexes for the same pattern
    let regex = REGEX_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache
            .entry(pattern.clone())
            .or_insert_with(|| {
                Regex::new(&pattern).expect("escaped basename produces a valid regex")
            })
            .clone()
    });

    let mut found_correct = false;
    let mut has_broken_path = false;

    // Collect all matches first to avoid false positives
    // Use captures_iter to directly extract the captured path group
    for captures in regex.captures_iter(&normalized_body) {
        let candidate = captures.get(1).map(|m| m.as_str()).unwrap_or("");

        // Writers commonly prefix skill-relative paths with one or more `./`
        // segments. Those are equivalent to the plain relative path, so they
        // must not be reported as broken references.
        let candidate_clean = candidate.trim_start_matches("./");

        if candidate_clean == path_str_posix {
            found_correct = true;
        } else {
            has_broken_path = true;
        }
    }

    // Report BrokenPath if ANY broken reference exists, even if correct ones also exist
    // This ensures broken links are not silently ignored
    if has_broken_path {
        ReferenceStatus::BrokenPath
    } else if found_correct {
        ReferenceStatus::Found
    } else {
        ReferenceStatus::Orphaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_frontmatter_returns_metadata_and_body() {
        let parsed = parse_frontmatter("---\nname: my-skill\ndescription: desc\n---\n# Body\n")
            .expect("parses");

        assert_eq!(
            parsed.metadata.get("name"),
            Some(&Value::String("my-skill".to_string()))
        );
        assert_eq!(parsed.body, "# Body");
    }
}
