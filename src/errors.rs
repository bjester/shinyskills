// Copyright 2026 Blaine Jester
// SPDX-License-Identifier: MIT
//
// Derived from agentskills/skills-ref (Apache-2.0)
use thiserror::Error;

/// Error types for skill-related operations.
#[derive(Error, Debug)]
pub enum SkillError {
    /// Error parsing SKILL.md or YAML frontmatter.
    #[error("{0}")]
    ParseError(String),

    /// Error validating skill properties.
    #[error("{0}")]
    ValidationError(String),
}
