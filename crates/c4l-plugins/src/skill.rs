//! Skill system: load SKILL.md / README.md files with YAML frontmatter.
//!
//! Format (from Superpowers):
//! ---
//! name: skill-name
//! description: "Use when [conditions]"
//! ---
//! [markdown content]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Skill manifest parsed from frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    pub name: String,
    pub description: String,
    /// Full markdown content (loaded on demand).
    #[serde(skip)]
    pub content: Option<String>,
    /// Source file path.
    #[serde(skip)]
    pub path: Option<PathBuf>,
}

/// Parse a skill file (SKILL.md or README.md) with YAML frontmatter.
pub fn parse_skill_file(path: &Path) -> Result<SkillManifest> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read skill file: {}", path.display()))?;

    let (frontmatter, content) = split_frontmatter(&raw)
        .ok_or_else(|| anyhow::anyhow!("no YAML frontmatter found in {}", path.display()))?;

    let mut manifest: SkillManifest = serde_yaml_frontmatter(&frontmatter)?;
    manifest.content = Some(content.to_string());
    manifest.path = Some(path.to_path_buf());

    Ok(manifest)
}

/// Split markdown into YAML frontmatter and body content.
/// Frontmatter is delimited by "---" on its own line.
fn split_frontmatter(text: &str) -> Option<(&str, &str)> {
    let text = text.trim_start();
    if !text.starts_with("---") {
        return None;
    }

    let after_first = &text[3..];
    let end = after_first.find("\n---")?;

    let frontmatter = after_first[..end].trim();
    let content = after_first[end + 4..].trim_start_matches(['\n', '\r']);

    Some((frontmatter, content))
}

/// Parse YAML frontmatter into a SkillManifest.
/// We do a simple key-value parse to avoid adding a YAML dependency.
fn serde_yaml_frontmatter(yaml: &str) -> Result<SkillManifest> {
    let mut name = None;
    let mut description = None;

    for line in yaml.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("name:") {
            name = Some(value.trim().trim_matches('"').trim_matches('\'').to_string());
        } else if let Some(value) = line.strip_prefix("description:") {
            description = Some(value.trim().trim_matches('"').trim_matches('\'').to_string());
        }
    }

    Ok(SkillManifest {
        name: name.ok_or_else(|| anyhow::anyhow!("missing 'name' in frontmatter"))?,
        description: description.ok_or_else(|| anyhow::anyhow!("missing 'description' in frontmatter"))?,
        content: None,
        path: None,
    })
}

/// Discover skills from a list of directories.
///
/// Scans for SKILL.md or README.md files in each directory.
/// Search pattern: <dir>/<skill-name>/SKILL.md or <dir>/<skill-name>/README.md
pub fn discover_skills(dirs: &[PathBuf]) -> Vec<SkillManifest> {
    let mut skills = Vec::new();

    for dir in dirs {
        if !dir.exists() {
            continue;
        }

        for entry in WalkDir::new(dir).max_depth(2).into_iter().flatten() {
            let path = entry.path();
            let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");

            if filename == "SKILL.md" || (filename == "README.md" && path.parent() != Some(dir.as_path())) {
                match parse_skill_file(path) {
                    Ok(skill) => skills.push(skill),
                    Err(e) => {
                        tracing::warn!(?path, %e, "failed to parse skill file");
                    }
                }
            }
        }
    }

    skills
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_frontmatter() {
        let content = "---\nname: test-skill\ndescription: \"Use when testing\"\n---\n\n# Test Skill\n\nThis is the content.\n";
        let (fm, body) = split_frontmatter(content).unwrap();
        assert!(fm.contains("test-skill"));
        assert!(body.contains("# Test Skill"));
    }

    #[test]
    fn parse_skill_manifest() {
        let yaml = "name: my-skill\ndescription: Use when doing X";
        let manifest = serde_yaml_frontmatter(yaml).unwrap();
        assert_eq!(manifest.name, "my-skill");
        assert_eq!(manifest.description, "Use when doing X");
    }

    #[test]
    fn parse_quoted_values() {
        let yaml = "name: \"quoted-name\"\ndescription: 'single quoted'";
        let manifest = serde_yaml_frontmatter(yaml).unwrap();
        assert_eq!(manifest.name, "quoted-name");
        assert_eq!(manifest.description, "single quoted");
    }

    #[test]
    fn parse_real_skill_file() {
        let dir = tempfile::tempdir().unwrap();
        let skill_dir = dir.path().join("my-skill");
        std::fs::create_dir(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: \"Use when testing\"\n---\n\n# My Skill\n\nContent here.\n",
        ).unwrap();

        let manifest = parse_skill_file(&skill_dir.join("SKILL.md")).unwrap();
        assert_eq!(manifest.name, "my-skill");
        assert!(manifest.content.unwrap().contains("# My Skill"));
    }

    #[test]
    fn discover_skills_from_dir() {
        let dir = tempfile::tempdir().unwrap();

        // Create two skill directories
        let s1 = dir.path().join("skill-a");
        std::fs::create_dir(&s1).unwrap();
        std::fs::write(s1.join("SKILL.md"), "---\nname: skill-a\ndescription: A\n---\nContent A").unwrap();

        let s2 = dir.path().join("skill-b");
        std::fs::create_dir(&s2).unwrap();
        std::fs::write(s2.join("SKILL.md"), "---\nname: skill-b\ndescription: B\n---\nContent B").unwrap();

        // Not a skill (no frontmatter)
        std::fs::write(dir.path().join("not-a-skill.md"), "just a file").unwrap();

        let skills = discover_skills(&[dir.path().to_path_buf()]);
        assert_eq!(skills.len(), 2);
    }

    #[test]
    fn no_frontmatter_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("bad.md");
        std::fs::write(&file, "# No frontmatter here\n").unwrap();
        assert!(parse_skill_file(&file).is_err());
    }
}
