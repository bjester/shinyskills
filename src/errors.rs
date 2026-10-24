// Copyright 2026 Blaine Jester
// SPDX-License-Identifier: MIT
//
// Derived from agentskills/skills-ref (Apache-2.0)
//! Error types for skill parsing, validation, and configuration.
use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Errors returned while parsing, validating, and configuring skills.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum SkillError {
    /// Error parsing SKILL.md or YAML frontmatter.
    #[error("{0}")]
    ParseError(String),

    /// Error validating skill properties.
    #[error("{0}")]
    ValidationError(String),

    /// Configuration file was explicitly requested but does not exist.
    #[error("Configuration file not found: {path}")]
    ConfigNotFound {
        /// Path that was requested.
        path: PathBuf,
    },

    /// Configuration file exists but could not be read.
    #[error("Failed to read configuration file {}: {source}", path.display())]
    ConfigRead {
        /// Path that could not be read.
        path: PathBuf,
        /// Underlying filesystem error.
        source: io::Error,
    },

    /// Configuration file is not valid TOML.
    ///
    /// The message preserves the source line and column reported by the parser.
    #[error("Invalid configuration in {path}: {message}")]
    ConfigParse {
        /// Path of the malformed file.
        path: PathBuf,
        /// Parser message including line and column information.
        message: String,
    },

    /// An `ignore_files` entry is not a valid glob pattern.
    #[error("Invalid ignore_files glob `{pattern}`: {source}")]
    InvalidIgnoreGlob {
        /// The offending pattern.
        pattern: String,
        /// Underlying glob compilation error.
        source: globset::Error,
    },
}
