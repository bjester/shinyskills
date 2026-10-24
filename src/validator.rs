// Copyright 2026 Blaine Jester
// SPDX-License-Identifier: MIT
//
// Derived from agentskills/skills-ref (Apache-2.0)
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;

use crate::models::{ParsedFrontmatter, ValidationOptions};
use crate::parser::{
    find_skill_md, is_path_referenced, parse_frontmatter, scan_skill_files, ReferenceStatus,
};

// Re-exported so `shinyskills::validator::ValidationMode` keeps resolving.
pub use crate::models::ValidationMode;

const MAX_SKILL_NAME_LENGTH: usize = 64;
const MAX_DESCRIPTION_LENGTH: usize = 1024;
const MAX_COMPATIBILITY_LENGTH: usize = 500;

/// Maximum file size to read into memory (1MB) to prevent OOM errors
const MAX_FILE_SIZE: u64 = 1024 * 1024;

fn validate_name(name: &str, skill_dir: Option<&Path>) -> Vec<String> {
    let mut errors = Vec::new();

    if name.is_empty() {
        errors.push("Field 'name' must be a non-empty string".to_string());
        return errors;
    }

    let normalized_name = name.nfkc().collect::<String>();

    if normalized_name.len() > MAX_SKILL_NAME_LENGTH {
        errors.push(format!(
            "Skill name '{}' exceeds {} character limit ({} chars)",
            normalized_name,
            MAX_SKILL_NAME_LENGTH,
            normalized_name.len()
        ));
    }

    // Check for invalid characters first (before lowercase check)
    if !normalized_name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-')
    {
        errors.push(format!(
            "Skill name '{}' contains invalid characters. Only letters, digits, and hyphens are allowed.",
            normalized_name
        ));
    }

    if normalized_name != normalized_name.to_lowercase() {
        errors.push(format!(
            "Skill name '{}' must be lowercase",
            normalized_name
        ));
    }

    if normalized_name.starts_with('-') || normalized_name.ends_with('-') {
        errors.push("Skill name cannot start or end with a hyphen".to_string());
    }

    if normalized_name.contains("--") {
        errors.push("Skill name cannot contain consecutive hyphens".to_string());
    }

    if let Some(skill_dir) = skill_dir {
        let dir_name = skill_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let normalized_dir_name = dir_name.nfkc().collect::<String>();
        if normalized_dir_name != normalized_name {
            errors.push(format!(
                "Directory name '{}' must match skill name '{}'",
                dir_name, normalized_name
            ));
        }
    }

    errors
}

fn validate_description(description: &str) -> Vec<String> {
    let mut errors = Vec::new();

    if description.is_empty() {
        errors.push("Field 'description' must be a non-empty string".to_string());
        return errors;
    }

    if description.len() > MAX_DESCRIPTION_LENGTH {
        errors.push(format!(
            "Description exceeds {} character limit ({} chars)",
            MAX_DESCRIPTION_LENGTH,
            description.len()
        ));
    }

    errors
}

fn validate_compatibility(compatibility: &str) -> Vec<String> {
    let mut errors = Vec::new();

    if compatibility.len() > MAX_COMPATIBILITY_LENGTH {
        errors.push(format!(
            "Compatibility exceeds {} character limit ({} chars)",
            MAX_COMPATIBILITY_LENGTH,
            compatibility.len()
        ));
    }

    errors
}

fn validate_metadata_fields(
    metadata: &HashMap<String, serde_json::Value>,
    options: &ValidationOptions,
) -> Vec<String> {
    let mut errors = Vec::new();

    let mut extra_fields: Vec<&String> = metadata
        .keys()
        .filter(|k| !options.allows_field(k))
        .collect();
    extra_fields.sort();

    if !extra_fields.is_empty() {
        errors.push(format!(
            "Unexpected fields in frontmatter: {}. Only {:?} are allowed.",
            extra_fields
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            options.allowed_field_names()
        ));
    }

    errors
}

/// Validate parsed skill metadata against resolved options.
///
/// This is the core validation function that works on already-parsed metadata,
/// avoiding duplicate file I/O when called from the parser.
///
/// # Arguments
/// * `metadata` - Parsed YAML frontmatter dictionary
/// * `skill_dir` - Optional path to skill directory (for name-directory match check)
/// * `options` - Resolved validation options determining allowed fields
///
/// # Returns
/// List of validation error messages. Empty list means valid.
pub fn validate_metadata_with_options(
    metadata: &HashMap<String, serde_json::Value>,
    skill_dir: Option<&Path>,
    options: &ValidationOptions,
) -> Vec<String> {
    let mut errors = Vec::new();
    errors.extend(validate_metadata_fields(metadata, options));

    // Validate name field
    match metadata.get("name") {
        Some(serde_json::Value::String(name)) => {
            errors.extend(validate_name(name, skill_dir));
        }
        Some(_) => {
            errors.push("Field 'name' must be a non-empty string".to_string());
        }
        None => {
            errors.push("Missing required field in frontmatter: name".to_string());
        }
    }

    // Validate description field
    match metadata.get("description") {
        Some(serde_json::Value::String(desc)) => {
            errors.extend(validate_description(desc));
        }
        Some(_) => {
            errors.push("Field 'description' must be a non-empty string".to_string());
        }
        None => {
            errors.push("Missing required field in frontmatter: description".to_string());
        }
    }

    // Validate optional compatibility field
    if let Some(serde_json::Value::String(compat)) = metadata.get("compatibility") {
        errors.extend(validate_compatibility(compat));
    }

    errors
}

/// Validate parsed skill metadata for a validation mode.
///
/// Kept for compatibility with callers that do not resolve configuration. It
/// delegates to [`validate_metadata_with_options`] using the mode's built-in
/// rules, which is what a run without any `.shinyskills.toml` produces.
pub fn validate_metadata(
    metadata: &HashMap<String, serde_json::Value>,
    skill_dir: Option<&Path>,
    mode: ValidationMode,
) -> Vec<String> {
    match crate::config::resolve_validation_options(None, mode) {
        Ok(options) => validate_metadata_with_options(metadata, skill_dir, &options),
        Err(e) => vec![e.to_string()],
    }
}

/// Record how `current_file`'s content refers to each of `files`.
///
fn track_reference_status(
    files: &[PathBuf],
    current_file: &Path,
    body: &str,
    file_statuses: &mut HashMap<PathBuf, HashSet<ReferenceStatus>>,
    all_errors: &mut Vec<String>,
) {
    for rel_path in files {
        if rel_path == current_file {
            continue;
        }
        let result = is_path_referenced(body, rel_path);
        file_statuses
            .entry(rel_path.clone())
            .or_default()
            .insert(result);
        if result == ReferenceStatus::BrokenPath {
            all_errors.push(format!(
                "File {} has broken path reference to: {}",
                current_file.display(),
                rel_path.display()
            ));
        }
    }
}

/// Validate references in SKILL.md and all other skill files.
///
/// Tracks orphaned files across ALL files - a file is only orphaned if it's not
/// referenced in ANY skill file (SKILL.md or other referenced files). Files
/// matched by `ignore_files` take no part in reference analysis.
///
/// # Arguments
/// * `skill_dir` - Path to the skill directory
/// * `body` - Markdown body content from SKILL.md
/// * `options` - Resolved validation options for ignored and scannable files
///
/// # Returns
/// Tuple of (warnings, errors):
/// - warnings: Files orphaned across all skill files
/// - errors: Broken path references from any file
pub fn validate_skill_references_with_options(
    skill_dir: &Path,
    body: &str,
    options: &ValidationOptions,
) -> Result<(Vec<String>, Vec<String>), std::io::Error> {
    let files: Vec<PathBuf> = scan_skill_files(skill_dir)?
        .into_iter()
        .filter(|rel_path| !options.is_ignored(rel_path))
        .collect();

    let mut file_statuses: HashMap<PathBuf, HashSet<ReferenceStatus>> = HashMap::new();
    for f in &files {
        file_statuses.insert(f.clone(), HashSet::new());
    }

    let mut all_errors: Vec<String> = Vec::new();

    if let Some(skill_md) = find_skill_md(skill_dir) {
        let current_file = PathBuf::from(
            skill_md
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("SKILL.md"),
        );
        track_reference_status(
            &files,
            &current_file,
            body,
            &mut file_statuses,
            &mut all_errors,
        );
    }

    for rel_path in &files {
        if !options.is_scannable(rel_path) {
            continue;
        }

        let file_path = skill_dir.join(rel_path);

        // Check file size before reading to prevent OOM
        let metadata = match fs::metadata(&file_path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if metadata.len() > MAX_FILE_SIZE {
            // Skip large files - they likely contain binary data
            continue;
        }

        if let Ok(content) = fs::read_to_string(&file_path) {
            if content.contains('\0') {
                continue;
            }
            track_reference_status(
                &files,
                rel_path,
                &content,
                &mut file_statuses,
                &mut all_errors,
            );
        }
    }

    let mut all_warnings: Vec<String> = Vec::new();
    for rel_path in &files {
        if let Some(statuses) = file_statuses.get(rel_path) {
            if !statuses.contains(&ReferenceStatus::Found)
                && !statuses.contains(&ReferenceStatus::BrokenPath)
            {
                all_warnings.push(format!(
                    "Orphaned file (not referenced): {}",
                    rel_path.display()
                ));
            }
        }
    }

    Ok((all_warnings, all_errors))
}

/// Validate a skill directory with resolved options.
///
/// Performs comprehensive validation including:
/// - Frontmatter structure and required fields
/// - Skill name format and directory match
/// - File references (orphaned files and broken paths)
///
/// # Arguments
/// * `skill_dir` - Path to the skill directory
/// * `options` - Resolved validation options for this run
///
/// # Returns
/// Tuple of (warnings, errors):
/// - warnings: Orphaned files (not referenced in any skill file)
/// - errors: Validation errors (invalid metadata, broken paths)
pub fn validate_with_options(
    skill_dir: &Path,
    options: &ValidationOptions,
) -> (Vec<String>, Vec<String>) {
    let skill_dir = PathBuf::from(skill_dir);

    if !skill_dir.exists() {
        return (
            Vec::new(),
            vec![format!("Path does not exist: {}", skill_dir.display())],
        );
    }

    if !skill_dir.is_dir() {
        return (
            Vec::new(),
            vec![format!("Not a directory: {}", skill_dir.display())],
        );
    }

    let skill_md = match find_skill_md(&skill_dir) {
        Some(path) => path,
        None => {
            return (
                Vec::new(),
                vec!["Missing required file: SKILL.md".to_string()],
            )
        }
    };

    let content = match fs::read_to_string(&skill_md) {
        Ok(c) => c,
        Err(e) => return (Vec::new(), vec![format!("Failed to read SKILL.md: {}", e)]),
    };

    let ParsedFrontmatter { metadata, body } = match parse_frontmatter(&content) {
        Ok(parsed) => parsed,
        Err(e) => return (Vec::new(), vec![e.to_string()]),
    };

    let errors = validate_metadata_with_options(&metadata, Some(&skill_dir), options);

    match validate_skill_references_with_options(&skill_dir, &body, options) {
        Ok((ref_warnings, ref_errors)) => {
            let mut all_errors = errors;
            all_errors.extend(ref_errors);
            (ref_warnings, all_errors)
        }
        Err(e) => (
            Vec::new(),
            vec![format!("Error scanning skill files: {}", e)],
        ),
    }
}

/// Validate a skill directory for a validation mode without configuration.
///
/// Resolves the mode's built-in options and delegates to
/// [`validate_with_options`]. Callers that have repository configuration should
/// resolve options themselves so discovery happens once per repository.
pub fn validate(skill_dir: &Path, mode: ValidationMode) -> (Vec<String>, Vec<String>) {
    match crate::config::resolve_validation_options(None, mode) {
        Ok(options) => validate_with_options(skill_dir, &options),
        Err(e) => (Vec::new(), vec![e.to_string()]),
    }
}

/// Validate references using the built-in rules for the default mode.
///
/// Kept for callers that do not resolve configuration; equivalent to calling
/// [`validate_skill_references_with_options`] with baseline options.
pub fn validate_skill_references(
    skill_dir: &Path,
    body: &str,
) -> Result<(Vec<String>, Vec<String>), std::io::Error> {
    let options = match crate::config::resolve_validation_options(None, ValidationMode::Default) {
        Ok(options) => options,
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, e)),
    };
    validate_skill_references_with_options(skill_dir, body, &options)
}
