// Copyright 2026 Blaine Jester
// SPDX-License-Identifier: MIT
//
// Derived from agentskills/skills-ref (Apache-2.0)
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::errors::SkillError;
use crate::models::SkillProperties;

const EXCLUDED_REFERENCE_NAMES: &[&str] = &["venv", "__pycache__", "node_modules"];

thread_local! {
    static REGEX_CACHE: RefCell<HashMap<String, regex::Regex>> = RefCell::new(HashMap::new());
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
        if let Some(ctx) = context_dir {
            if !parent.starts_with(ctx) {
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
/// Tuple of (metadata hashmap, markdown body)
///
/// # Errors
/// Returns `ParseError` if frontmatter is missing, malformed, or contains invalid YAML.
pub fn parse_frontmatter(
    content: &str,
) -> Result<(HashMap<String, serde_json::Value>, String), SkillError> {
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

    let frontmatter_str = parts[1];
    let body = parts[2].trim().to_string();

    let metadata: HashMap<String, serde_json::Value> = serde_yaml::from_str(frontmatter_str)
        .map_err(|e| SkillError::ParseError(format!("Invalid YAML in frontmatter: {}", e)))?;

    Ok((metadata, body))
}

/// Extract an optional string field from metadata.
fn extract_string_field(
    metadata: &HashMap<String, serde_json::Value>,
    key: &str,
) -> Option<String> {
    match metadata.get(key) {
        Some(serde_json::Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

/// Extract the metadata sub-mapping as a HashMap<String, String>.
fn extract_metadata_field(
    metadata: &HashMap<String, serde_json::Value>,
) -> Option<HashMap<String, String>> {
    match metadata.get("metadata") {
        Some(serde_json::Value::Object(obj)) => {
            let mut map = HashMap::new();
            for (k, v) in obj {
                let value = match v {
                    serde_json::Value::String(s) => s.clone(),
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
    let skill_dir = PathBuf::from(skill_dir);
    let skill_md = match find_skill_md(&skill_dir) {
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

    let (metadata, _) = parse_frontmatter(&content)?;

    let name = match metadata.get("name") {
        Some(serde_json::Value::String(n)) => n.clone(),
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
        Some(serde_json::Value::String(d)) => d.clone(),
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
) -> Result<Vec<PathBuf>, std::io::Error> {
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

        let rel_path = path.strip_prefix(relative_to).unwrap_or(&path);

        if path.is_dir() {
            let mut sub_files = recurse_scan_skill_files(&path, relative_to)?;
            files.append(&mut sub_files);
            continue;
        }

        if file_name.to_lowercase() == "skill.md" {
            continue;
        }

        files.push(rel_path.to_path_buf());
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
pub fn scan_skill_files(skill_dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let files = recurse_scan_skill_files(skill_dir, skill_dir)?;
    Ok(files)
}

/// Status of a file reference check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceStatus {
    Found,
    Orphaned,
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
/// * `rel_path` - Relative path to search for (e.g., Path('references/GUIDE.md'))
///
/// # Returns
/// `ReferenceStatus`:
/// - `Found`: All references to this file use the correct path
/// - `BrokenPath`: At least one reference uses a wrong path (e.g., docs/X.md vs references/X.md)
/// - `Orphaned`: File is not referenced at all
pub fn is_path_referenced(body: &str, rel_path: &Path) -> ReferenceStatus {
    let path_str_posix = rel_path.to_string_lossy().replace('\\', "/");
    let path_str_basename = rel_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

    let normalized_body = body.replace('\\', "/");

    // Build pattern to match the basename with optional directory prefix
    let pattern = format!(
        r"(?:^|[^A-Za-z0-9._/-])((?:[A-Za-z0-9._-]*/)*{})(?:$|[^A-Za-z0-9._/-])",
        regex::escape(path_str_basename)
    );

    // Use thread-local cache to avoid recompiling regexes for the same pattern
    let re = REGEX_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache
            .entry(pattern.clone())
            .or_insert_with(|| regex::Regex::new(&pattern).unwrap())
            .clone()
    });

    let mut found_correct = false;
    let mut has_broken_path = false;

    // Collect all matches first to avoid false positives
    for mat in re.find_iter(&normalized_body) {
        // Extract just the path part (group 1 is the captured path)
        let candidate = mat.as_str();

        // Remove leading non-path character if present
        let candidate =
            if candidate.len() > 1 && !candidate.chars().next().unwrap().is_alphanumeric() {
                &candidate[1..]
            } else {
                candidate
            };

        // Remove trailing non-path character if present
        let candidate =
            candidate.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '/');

        if candidate == path_str_posix {
            found_correct = true;
        } else {
            has_broken_path = true;
        }
    }

    if found_correct {
        ReferenceStatus::Found
    } else if has_broken_path {
        ReferenceStatus::BrokenPath
    } else {
        ReferenceStatus::Orphaned
    }
}
