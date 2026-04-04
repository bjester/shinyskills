// Copyright 2026 Blaine Jester
// SPDX-License-Identifier: MIT
//
// Derived from agentskills/skills-ref (Apache-2.0)
use std::path::Path;

use crate::parser::{find_skill_md, read_properties};

/// Generate the <available_skills> XML block for inclusion in agent prompts.
///
/// This XML format is what Anthropic uses and recommends for Claude models.
/// Skill Clients may format skill information differently to suit their
/// models or preferences.
///
/// # Arguments
/// * `skill_dirs` - List of paths to skill directories
///
/// # Returns
/// XML string with <available_skills> block containing each skill's
/// name, description, and location.
///
/// Example output:
/// ```xml
/// <available_skills>
/// <skill>
/// <name>pdf-reader</name>
/// <description>Read and extract text from PDF files</description>
/// <location>/path/to/pdf-reader/SKILL.md</location>
/// </skill>
/// </available_skills>
/// ```
pub fn to_prompt(skill_dirs: &[&Path]) -> String {
    if skill_dirs.is_empty() {
        return "<available_skills>\n</available_skills>".to_string();
    }

    let mut lines = vec!["<available_skills>".to_string()];

    for skill_dir in skill_dirs {
        let props = match read_properties(skill_dir) {
            Ok(p) => p,
            Err(_) => continue,
        };

        lines.push("<skill>".to_string());
        lines.push(format!("<name>{}</name>", escape_xml(&props.name)));
        lines.push(format!(
            "<description>{}</description>",
            escape_xml(&props.description)
        ));

        if let Some(skill_md) = find_skill_md(skill_dir) {
            lines.push(format!("<location>{}</location>", skill_md.display()));
        }

        lines.push("</skill>".to_string());
    }

    lines.push("</available_skills>".to_string());

    lines.join("\n")
}

fn escape_xml(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&apos;"),
            _ => result.push(c),
        }
    }
    result
}
