// Copyright 2026 Blaine Jester
// SPDX-License-Identifier: MIT
//
// Derived from agentskills/skills-ref (Apache-2.0)
//! Repository configuration file support (`.shinyskills.toml`).
use globset::{Glob, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::errors::SkillError;
use crate::models::{ValidationMode, ValidationOptions};

/// Name of the repository configuration file discovered automatically.
pub const CONFIG_FILE_NAME: &str = ".shinyskills.toml";

/// Root of a `.shinyskills.toml` configuration file.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Config {
    /// Validation-related settings.
    #[serde(default)]
    pub validation: ValidationConfig,
}

/// Validation settings read from `.shinyskills.toml`.
///
/// The `Default` implementation is explicit so that Rust defaults stay in
/// sync with the Serde field defaults.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct ValidationConfig {
    /// Extra frontmatter keys accepted beyond the built-in set.
    #[serde(default = "default_allowed_fields")]
    pub allowed_fields: Vec<String>,

    /// Glob patterns for files excluded from reference scanning.
    #[serde(default)]
    pub ignore_files: Vec<String>,

    /// File extensions whose contents are scanned for references.
    #[serde(default = "default_scan_extensions")]
    pub scan_extensions: Vec<String>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            allowed_fields: default_allowed_fields(),
            ignore_files: Vec::new(),
            scan_extensions: default_scan_extensions(),
        }
    }
}

/// Frontmatter fields accepted by the default validation mode.
///
/// Delegates to [`ValidationMode::allowed_fields`] so the specification stays
/// the single source of truth for built-in keys.
fn default_allowed_fields() -> Vec<String> {
    ValidationMode::Default
        .allowed_fields()
        .iter()
        .map(|field| (*field).to_string())
        .collect()
}

/// Only markdown files are scanned for reference syntax by default.
fn default_scan_extensions() -> Vec<String> {
    vec![".md".to_string()]
}

/// Find the repository root that owns `start_path`.
///
/// Walks upward from `start_path` until a `.git` directory *or* file is found
/// (worktrees store `.git` as a file). Skill directories and intermediate
/// ancestors are never inspected for configuration, only for `.git`.
///
/// The starting path is lexically normalized before the walk, so `.` and `..`
/// spellings of the same directory resolve to the same root.
///
/// # Arguments
/// * `start_path` - A file or directory inside the repository
///
/// # Returns
/// The repository root, or `None` when no `.git` ancestor exists.
#[must_use]
pub fn find_repository_root(start_path: &Path) -> Option<PathBuf> {
    let absolute = absolute_path(start_path);

    let mut current = if absolute.is_dir() {
        Some(absolute.as_path())
    } else {
        absolute.parent()
    };

    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }

    None
}

/// Locate the single automatic configuration file for the repository that owns
/// `start_path`.
///
/// Returns `None` when there is no `.git` ancestor or when the repository root
/// contains no `.shinyskills.toml`. Nested configuration files are intentionally
/// ignored; only `<repository-root>/.shinyskills.toml` is ever used.
#[must_use]
pub fn find_repository_config(start_path: &Path) -> Option<PathBuf> {
    let root = find_repository_root(start_path)?;
    let config_path = root.join(CONFIG_FILE_NAME);
    if config_path.is_file() {
        Some(config_path)
    } else {
        None
    }
}

/// Read and parse a configuration file from an explicit path.
///
/// # Arguments
/// * `path` - Path to a `.shinyskills.toml` file
///
/// # Errors
/// * `ConfigNotFound` - No file exists at `path`
/// * `ConfigRead` - The file exists but cannot be read
/// * `ConfigParse` - The file is not valid TOML (message keeps line/column)
pub fn load_config(path: &Path) -> Result<Config, SkillError> {
    if !path.exists() {
        return Err(SkillError::ConfigNotFound {
            path: path.to_path_buf(),
        });
    }

    let content = fs::read_to_string(path).map_err(|source| SkillError::ConfigRead {
        path: path.to_path_buf(),
        source,
    })?;

    toml::from_str(&content).map_err(|e| SkillError::ConfigParse {
        path: path.to_path_buf(),
        message: e.to_string(),
    })
}

/// Build validation options from an optional configuration and a mode.
///
/// Allowed frontmatter fields are the union of the mode's built-in fields and
/// any additional fields declared by the configuration. Glob patterns are
/// compiled once, and scan extensions are normalized to lowercase with a
/// leading dot.
///
/// # Errors
/// * `InvalidIgnoreGlob` - An `ignore_files` pattern cannot be compiled
pub fn resolve_validation_options(
    config: Option<&Config>,
    mode: ValidationMode,
) -> Result<ValidationOptions, SkillError> {
    let defaults = ValidationConfig::default();
    let validation = config.map_or(&defaults, |config| &config.validation);

    let mut allowed_fields: HashSet<String> = HashSet::new();
    allowed_fields.extend(mode.allowed_fields().iter().map(|field| field.to_string()));
    allowed_fields.extend(validation.allowed_fields.iter().cloned());

    let mut builder = GlobSetBuilder::new();
    for pattern in &validation.ignore_files {
        let glob = Glob::new(pattern).map_err(|source| SkillError::InvalidIgnoreGlob {
            pattern: pattern.clone(),
            source,
        })?;
        builder.add(glob);
    }
    let ignore_files = builder
        .build()
        .map_err(|source| SkillError::InvalidIgnoreGlob {
            pattern: validation.ignore_files.join(", "),
            source,
        })?;

    let scan_extensions = normalize_scan_extensions(&validation.scan_extensions);

    Ok(ValidationOptions::new(
        allowed_fields,
        ignore_files,
        scan_extensions,
    ))
}

/// Lowercase scan extensions and make sure each one starts with a dot.
fn normalize_scan_extensions(extensions: &[String]) -> Vec<String> {
    extensions
        .iter()
        .map(|raw| {
            let trimmed = raw.trim().to_lowercase();
            if trimmed.starts_with('.') {
                trimmed
            } else {
                format!(".{}", trimmed)
            }
        })
        .collect()
}

/// Best-effort absolute form of `path`, used only for upward directory walks.
///
/// `.` and `..` components are collapsed so the upward walk sees the same path
/// however the caller spelled it, and the repository root stays a stable key for
/// per-run caching.
fn absolute_path(path: &Path) -> PathBuf {
    match std::env::current_dir() {
        Ok(current_dir) if !path.is_absolute() => normalize_components(&current_dir.join(path)),
        _ => normalize_components(path),
    }
}

/// Collapse `.` and `..` components lexically, clamping at the filesystem root.
///
/// This needs no filesystem access, so it works on paths that do not exist and
/// leaves symlinks unresolved. [`std::fs::canonicalize`] is deliberately avoided:
/// it resolves symlinks, fails on missing paths, and would change discovery
/// results for symlinked checkouts such as `/tmp` on macOS.
fn normalize_components(path: &Path) -> PathBuf {
    let mut root: Vec<&OsStr> = Vec::new();
    let mut stack: Vec<&OsStr> = Vec::new();

    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => root.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if stack.pop().is_some() {
                    // Consumed one real component above the directory.
                } else if root.is_empty() {
                    // A relative path may legitimately reach above the
                    // starting directory, so keep the unresolvable `..`.
                    stack.push(component.as_os_str());
                }
                // Otherwise clamp: absolute paths cannot escape the root.
            }
            Component::Normal(name) => stack.push(name),
        }
    }

    if root.is_empty() && stack.is_empty() {
        return PathBuf::from(".");
    }

    let mut normalized = PathBuf::new();
    let parts = root.iter().chain(stack.iter());
    for part in parts {
        normalized.push(part);
    }
    normalized
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn empty_toml_yields_default_config() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config, Config::default());
        assert_eq!(config.validation.allowed_fields, default_allowed_fields());
        assert_eq!(config.validation.scan_extensions, default_scan_extensions());
    }

    #[test]
    fn ignore_files_defaults_to_empty() {
        let config: Config = toml::from_str("[validation]\nallowed_fields = [\"name\"]\n").unwrap();
        assert!(config.validation.ignore_files.is_empty());

        let rust_default = ValidationConfig::default();
        assert!(rust_default.ignore_files.is_empty());
        // Rust defaults must agree with Serde field defaults.
        assert_eq!(rust_default.allowed_fields, default_allowed_fields());
        assert_eq!(rust_default.scan_extensions, default_scan_extensions());
        let serde_default: Config = toml::from_str("[validation]").unwrap();
        assert_eq!(rust_default, serde_default.validation);
    }

    #[test]
    fn custom_values_populate_validation_config() {
        let toml_src = "\
[validation]
allowed_fields = [\"name\", \"description\", \"user-invocable\"]
ignore_files = [\"CHANGELOG.md\", \"docs/**\"]
scan_extensions = [\".md\", \".markdown\"]
";
        let config: Config = toml::from_str(toml_src).unwrap();
        assert_eq!(
            config.validation.allowed_fields,
            ["name", "description", "user-invocable"]
        );
        assert_eq!(config.validation.ignore_files, ["CHANGELOG.md", "docs/**"]);
        assert_eq!(config.validation.scan_extensions, [".md", ".markdown"]);
    }

    #[test]
    fn unknown_config_keys_are_rejected() {
        let root_error = toml::from_str::<Config>("[validaton]\n").unwrap_err();
        assert!(
            root_error.to_string().contains("unknown field `validaton`"),
            "error: {}",
            root_error
        );

        let validation_error =
            toml::from_str::<Config>("[validation]\nignore_file = [\"CHANGELOG.md\"]\n")
                .unwrap_err();
        assert!(
            validation_error
                .to_string()
                .contains("unknown field `ignore_file`"),
            "error: {}",
            validation_error
        );
    }

    /// How the repository's `.git` entry is spelled.
    enum GitLayout {
        /// A normal checkout with `.git/` as a directory.
        Directory,
        /// A worktree with `.git` as a file pointing elsewhere.
        WorktreeFile,
    }

    /// Build `<root>/.git` (directory or worktree file) plus a skill directory.
    fn make_repo(root: &Path, git_layout: GitLayout) -> PathBuf {
        let skill_dir = root.join("skills").join("skill-a");
        fs::create_dir_all(&skill_dir).unwrap();
        match git_layout {
            GitLayout::Directory => {
                fs::create_dir_all(root.join(".git")).unwrap();
            }
            GitLayout::WorktreeFile => {
                fs::write(root.join(".git"), "gitdir: /elsewhere\n").unwrap();
            }
        }
        fs::write(skill_dir.join("SKILL.md"), "---\nname: skill-a\n---\n").unwrap();
        skill_dir
    }

    #[test]
    fn repository_root_found_from_skill_dir_and_skill_md() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().join("repo");
        let skill_dir = make_repo(&root, GitLayout::Directory);
        fs::write(root.join(".shinyskills.toml"), "[validation]\n").unwrap();

        assert_eq!(find_repository_root(&skill_dir), Some(root.clone()));
        assert_eq!(
            find_repository_root(&skill_dir.join("SKILL.md")),
            Some(root.clone())
        );
        assert_eq!(
            find_repository_config(&skill_dir),
            Some(root.join(".shinyskills.toml"))
        );
        assert_eq!(
            find_repository_config(&skill_dir.join("SKILL.md")),
            Some(root.join(".shinyskills.toml"))
        );
    }

    #[test]
    fn repository_root_found_from_worktree_git_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().join("repo");
        let skill_dir = make_repo(&root, GitLayout::WorktreeFile);
        fs::write(root.join(".shinyskills.toml"), "[validation]\n").unwrap();

        assert_eq!(find_repository_root(&skill_dir), Some(root.clone()));
        assert_eq!(
            find_repository_config(&skill_dir),
            Some(root.join(".shinyskills.toml"))
        );
    }

    #[test]
    fn nested_configs_are_ignored() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().join("repo");
        let skill_dir = make_repo(&root, GitLayout::Directory);
        fs::write(root.join(".shinyskills.toml"), "[validation]\n").unwrap();
        // Intermediate and skill-level configurations must never be discovered.
        fs::write(
            root.join("skills").join(".shinyskills.toml"),
            "[validation]\n",
        )
        .unwrap();
        fs::write(skill_dir.join(".shinyskills.toml"), "[validation]\n").unwrap();

        assert_eq!(
            find_repository_config(&skill_dir),
            Some(root.join(".shinyskills.toml"))
        );
    }

    #[test]
    fn no_config_without_git_ancestor() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().join("plain");
        let skill_dir = root.join("skill-a");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "---\nname: skill-a\n---\n").unwrap();

        assert_eq!(find_repository_config(&skill_dir), None);
    }

    #[test]
    fn baseline_options_ignore_nothing_and_scan_markdown_only() {
        let options = resolve_validation_options(None, ValidationMode::Default).unwrap();

        assert!(!options.is_ignored(Path::new("CHANGELOG.md")));
        assert!(options.is_scannable(Path::new("SKILL.md")));
        assert!(options.is_scannable(Path::new("references/GUIDE.MD")));
        assert!(!options.is_scannable(Path::new("scripts/run.sh")));
        assert!(!options.is_scannable(Path::new("tests/run.test.cjs")));
        assert!(options.allows_field("name"));
        assert!(!options.allows_field("user-invocable"));
    }

    #[test]
    fn configured_options_ignore_files_and_add_allowed_fields() {
        let config: Config = toml::from_str(
            "[validation]\n\
             allowed_fields = [\"name\", \"user-invocable\"]\n\
             ignore_files = [\"CHANGELOG.md\", \"docs/**\"]\n",
        )
        .unwrap();

        let options = resolve_validation_options(Some(&config), ValidationMode::Default).unwrap();

        assert!(options.is_ignored(Path::new("CHANGELOG.md")));
        assert!(options.is_ignored(Path::new("docs/nested/note.txt")));
        assert!(!options.is_ignored(Path::new("references/CHANGELOG.md")));
        assert!(options.allows_field("user-invocable"));
        assert!(options.allows_field("name"));
    }

    #[test]
    fn scan_extensions_are_normalized() {
        let config: Config =
            toml::from_str("[validation]\nscan_extensions = [\"MD\", \".sh\"]\n").unwrap();
        let options = resolve_validation_options(Some(&config), ValidationMode::Default).unwrap();

        assert!(options.is_scannable(Path::new("a.md")));
        assert!(options.is_scannable(Path::new("scripts/run.sh")));
        assert!(!options.is_scannable(Path::new("notes.txt")));
    }

    #[test]
    fn invalid_glob_reports_offending_pattern() {
        let config: Config = toml::from_str("[validation]\nignore_files = [\"[\"]\n").unwrap();
        let err = resolve_validation_options(Some(&config), ValidationMode::Default).unwrap_err();
        let SkillError::InvalidIgnoreGlob { pattern, .. } = &err else {
            panic!("unexpected error: {err:?}");
        };
        assert_eq!(pattern.as_str(), "[");
    }

    #[test]
    fn load_missing_config_reports_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join(".shinyskills.toml");
        let err = load_config(&path).unwrap_err();
        let SkillError::ConfigNotFound { path: err_path } = &err else {
            panic!("unexpected error: {err:?}");
        };
        assert_eq!(err_path, &path);
    }

    #[test]
    fn load_malformed_config_reports_path_and_position() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join(".shinyskills.toml");
        fs::write(&path, "this is not = valid = toml\n").unwrap();
        let err = load_config(&path).unwrap_err();
        let SkillError::ConfigParse {
            path: err_path,
            message,
        } = &err
        else {
            panic!("unexpected error: {err:?}");
        };
        assert_eq!(err_path, &path);
        assert!(message.contains("line"), "message: {}", message);
    }

    #[test]
    fn allowed_field_defaults_follow_default_validation_mode() {
        let mut expected: Vec<String> = ValidationMode::Default
            .allowed_fields()
            .iter()
            .map(|field| (*field).to_string())
            .collect();
        expected.sort();

        let mut actual = default_allowed_fields();
        actual.sort();

        assert_eq!(actual, expected);
    }

    #[test]
    fn normalize_components_collapses_dot_paths() {
        assert_eq!(
            normalize_components(Path::new("/a/b/.")),
            PathBuf::from("/a/b")
        );
        assert_eq!(
            normalize_components(Path::new("/a/b/..")),
            PathBuf::from("/a")
        );
        assert_eq!(normalize_components(Path::new("/..")), PathBuf::from("/"));
        assert_eq!(
            normalize_components(Path::new("/a/../..")),
            PathBuf::from("/")
        );
        assert_eq!(
            normalize_components(Path::new("a/./b/../c")),
            PathBuf::from("a/c")
        );
        assert_eq!(normalize_components(Path::new("./")), PathBuf::from("."));
        assert_eq!(
            normalize_components(Path::new("../a")),
            PathBuf::from("../a")
        );
    }

    #[test]
    fn repository_root_is_stable_across_dot_spellings() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().join("repo");
        let skill_dir = make_repo(&root, GitLayout::Directory);

        assert_eq!(
            find_repository_root(&skill_dir.join(".")),
            find_repository_root(&skill_dir),
            "the same directory must yield the same root however it is spelled"
        );
    }
}
