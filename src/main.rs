// Copyright 2026 Blaine Jester
// SPDX-License-Identifier: MIT
//
// Derived from agentskills/skills-ref (Apache-2.0)
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use shinyskills::config::{
    find_repository_config, find_repository_root, load_config, resolve_validation_options,
    CONFIG_FILE_NAME,
};
use shinyskills::errors::SkillError;
use shinyskills::parser::{find_skill_dir_for_file, read_properties};
use shinyskills::prompt::to_prompt;
use shinyskills::validator::{validate_with_options, ValidationMode};
use shinyskills::ValidationOptions;

#[derive(Parser)]
#[command(name = "shinyskills")]
#[command(author, version, about = "Reference library for Agent Skills", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a skill directory
    Validate {
        /// Path to the skill directory or SKILL.md file
        #[arg(value_name = "SKILL_PATH")]
        skill_path: PathBuf,

        /// Enable Vibe-specific validation rules (allows user-invocable in frontmatter)
        #[arg(long, default_value_t = false)]
        vibe: bool,

        /// Path to configuration file (.shinyskills.toml)
        #[arg(long, value_name = "CONFIG_PATH")]
        config: Option<PathBuf>,
    },
    /// Validate skill directories based on changed files (for pre-commit hooks)
    ValidateStaged {
        /// Changed files to check (typically from pre-commit)
        #[arg(value_name = "FILES")]
        files: Vec<PathBuf>,

        /// Enable Vibe-specific validation rules (allows user-invocable in frontmatter)
        #[arg(long, default_value_t = false)]
        vibe: bool,

        /// Context directory to limit upward search (don't search above this directory)
        #[arg(long, value_name = "DIR")]
        context_dir: Option<PathBuf>,

        /// Maximum levels to walk up when searching for skill directory
        #[arg(long, value_name = "N", default_value_t = 3)]
        max_depth: usize,

        /// Path to configuration file (.shinyskills.toml)
        #[arg(long, value_name = "CONFIG_PATH")]
        config: Option<PathBuf>,
    },
    /// Read and print skill properties as JSON
    ReadProperties {
        /// Path to the skill directory or SKILL.md file
        #[arg(value_name = "SKILL_PATH")]
        skill_path: PathBuf,
    },
    /// Generate <available_skills> XML for agent prompts
    ToPrompt {
        /// One or more skill directories or SKILL.md files
        #[arg(value_name = "SKILL_PATHS")]
        skill_paths: Vec<PathBuf>,
    },
}

fn is_skill_md_file(path: &std::path::Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.eq_ignore_ascii_case("skill.md"))
            .unwrap_or(false)
}

/// Resolve the validation options for one skill path.
///
/// An explicit `--config` path wins. Otherwise only the configuration next to
/// the repository root is used; skill directories and intermediate ancestors
/// are never consulted. A missing repository configuration means built-in
/// defaults, which keeps behaviour identical to a run before configuration
/// existed.
fn resolve_options_for_path(
    skill_dir: &Path,
    config: Option<&PathBuf>,
    mode: ValidationMode,
) -> Result<ValidationOptions, SkillError> {
    if let Some(explicit_path) = config {
        let loaded = load_config(explicit_path)?;
        return resolve_validation_options(Some(&loaded), mode);
    }

    match find_repository_config(skill_dir) {
        Some(discovered) => {
            let loaded = load_config(&discovered)?;
            resolve_validation_options(Some(&loaded), mode)
        }
        None => resolve_validation_options(None, mode),
    }
}

/// Caches resolved options so a multi-skill run reads each repository
/// configuration at most once.
struct OptionsCache {
    mode: ValidationMode,
    explicit: Option<ValidationOptions>,
    by_root: HashMap<Option<PathBuf>, ValidationOptions>,
}

impl OptionsCache {
    /// Build a cache from an optional explicit configuration.
    fn new(config: Option<&PathBuf>, mode: ValidationMode) -> Result<Self, SkillError> {
        let explicit = match config {
            Some(path) => {
                let loaded = load_config(path)?;
                Some(resolve_validation_options(Some(&loaded), mode)?)
            }
            None => None,
        };

        Ok(Self {
            mode,
            explicit,
            by_root: HashMap::new(),
        })
    }

    /// Options to use for a skill directory, resolved once per repository root.
    fn options_for(&mut self, skill_dir: &Path) -> Result<ValidationOptions, SkillError> {
        if let Some(options) = &self.explicit {
            return Ok(options.clone());
        }

        let root = find_repository_root(skill_dir);
        if let Some(options) = self.by_root.get(&root) {
            return Ok(options.clone());
        }

        // The root is already resolved above, so derive the configuration path
        // from it instead of walking for `.git` a second time.
        let loaded = match root.as_ref().map(|root| root.join(CONFIG_FILE_NAME)) {
            Some(path) if path.is_file() => Some(load_config(&path)?),
            _ => None,
        };
        let options = resolve_validation_options(loaded.as_ref(), self.mode)?;
        self.by_root.insert(root, options.clone());
        Ok(options)
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate {
            skill_path,
            vibe,
            config,
        } => {
            let skill_path = if is_skill_md_file(&skill_path) {
                skill_path
                    .parent()
                    .expect("SKILL.md file must have a parent directory")
                    .to_path_buf()
            } else {
                skill_path
            };

            let mode = if vibe {
                ValidationMode::Vibe
            } else {
                ValidationMode::Default
            };

            let options = match resolve_options_for_path(&skill_path, config.as_ref(), mode) {
                Ok(options) => options,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };
            let (warnings, errors) = validate_with_options(&skill_path, &options);

            if !warnings.is_empty() {
                eprintln!("Validation warnings for {}:", skill_path.display());
                for warning in warnings {
                    eprintln!("  - {}", warning);
                }
            }

            if !errors.is_empty() {
                eprintln!("Validation failed for {}:", skill_path.display());
                for error in errors {
                    eprintln!("  - {}", error);
                }
                std::process::exit(1);
            } else {
                println!("Valid skill: {}", skill_path.display());
            }
        }

        Commands::ValidateStaged {
            files,
            vibe,
            context_dir,
            max_depth,
            config,
        } => {
            let mode = if vibe {
                ValidationMode::Vibe
            } else {
                ValidationMode::Default
            };

            let mut cache = match OptionsCache::new(config.as_ref(), mode) {
                Ok(cache) => cache,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            };

            if files.is_empty() {
                // No files to check - silently succeed
                std::process::exit(0);
            }

            // Find unique skill directories from files
            let mut skill_dirs: BTreeSet<PathBuf> = BTreeSet::new();
            for file in &files {
                if let Some(skill_dir) =
                    find_skill_dir_for_file(file, context_dir.as_deref(), max_depth)
                {
                    skill_dirs.insert(skill_dir);
                }
            }

            if skill_dirs.is_empty() {
                // No skills affected - silently succeed
                std::process::exit(0);
            }

            // Validate each unique skill directory
            let mut total_warnings: Vec<String> = Vec::new();
            let mut total_errors: Vec<String> = Vec::new();

            for skill_dir in &skill_dirs {
                let options = match cache.options_for(skill_dir) {
                    Ok(options) => options,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                };
                let (warnings, errors) = validate_with_options(skill_dir, &options);

                // Prefix with skill name for clarity
                let skill_name = skill_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                for w in &warnings {
                    total_warnings.push(format!("[{}] {}", skill_name, w));
                }
                for e in &errors {
                    total_errors.push(format!("[{}] {}", skill_name, e));
                }
            }

            // Print results
            if !total_warnings.is_empty() {
                eprintln!("Validation warnings:");
                for warning in &total_warnings {
                    eprintln!("  - {}", warning);
                }
            }

            if !total_errors.is_empty() {
                eprintln!("Validation failed:");
                for error in &total_errors {
                    eprintln!("  - {}", error);
                }
                std::process::exit(1);
            } else {
                let skill_names: Vec<&str> = skill_dirs
                    .iter()
                    .map(|p| p.file_name().and_then(|n| n.to_str()).unwrap_or("unknown"))
                    .collect();
                println!("Valid skills: {}", skill_names.join(", "));
            }
        }

        Commands::ReadProperties { skill_path } => {
            let skill_path = if is_skill_md_file(&skill_path) {
                skill_path
                    .parent()
                    .expect("SKILL.md file must have a parent directory")
                    .to_path_buf()
            } else {
                skill_path
            };

            match read_properties(&skill_path) {
                Ok(props) => {
                    println!("{}", serde_json::to_string_pretty(&props.to_map()).unwrap());
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::ToPrompt { skill_paths } => {
            let mut resolved_paths = Vec::new();
            for path in skill_paths {
                if is_skill_md_file(&path) {
                    resolved_paths.push(
                        path.parent()
                            .expect("SKILL.md file must have a parent directory")
                            .to_path_buf(),
                    );
                } else {
                    resolved_paths.push(path);
                }
            }

            let path_refs: Vec<&std::path::Path> =
                resolved_paths.iter().map(|p| p.as_path()).collect();
            let output = to_prompt(&path_refs);
            println!("{}", output);
        }
    }
}
