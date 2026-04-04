// Copyright 2026 Blaine Jester
// SPDX-License-Identifier: MIT
//
// Derived from agentskills/skills-ref (Apache-2.0)
use std::collections::BTreeSet;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use shinyskills::parser::{find_skill_dir_for_file, read_properties};
use shinyskills::prompt::to_prompt;
use shinyskills::validator::{validate, ValidationMode};

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

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { skill_path, vibe } => {
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
            let (warnings, errors) = validate(&skill_path, mode);

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
        } => {
            if files.is_empty() {
                // No files to check - silently succeed
                std::process::exit(0);
            }

            let mode = if vibe {
                ValidationMode::Vibe
            } else {
                ValidationMode::Default
            };

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
                let (warnings, errors) = validate(skill_dir, mode);

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
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&props.to_dict()).unwrap()
                    );
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
