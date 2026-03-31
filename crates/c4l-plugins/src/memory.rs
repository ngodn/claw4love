//! Memory system: load CLAUDE.md files from project and user directories.
//!
//! Maps from: leak-claude-code/src/memdir/
//!
//! Hierarchy:
//! 1. Project root: ./CLAUDE.md
//! 2. Project config: .claude/CLAUDE.md
//! 3. User global: ~/.claude/CLAUDE.md

use std::path::{Path, PathBuf};
use tracing::debug;

/// A loaded memory file.
#[derive(Debug, Clone)]
pub struct MemoryFile {
    pub path: PathBuf,
    pub content: String,
    pub scope: MemoryScope,
}

/// Where a memory file comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryScope {
    /// ./CLAUDE.md in the project root
    Project,
    /// .claude/CLAUDE.md in the project
    ProjectConfig,
    /// ~/.claude/CLAUDE.md global user config
    UserGlobal,
}

/// Load all CLAUDE.md memory files in the hierarchy.
///
/// Returns them in priority order (project first, user last).
pub fn load_memory_files(project_root: &Path) -> Vec<MemoryFile> {
    let mut files = Vec::new();

    // 1. Project root: ./CLAUDE.md
    let project_md = project_root.join("CLAUDE.md");
    if let Some(mf) = try_load(&project_md, MemoryScope::Project) {
        files.push(mf);
    }

    // 2. Project config: .claude/CLAUDE.md
    let project_config_md = project_root.join(".claude").join("CLAUDE.md");
    if let Some(mf) = try_load(&project_config_md, MemoryScope::ProjectConfig) {
        files.push(mf);
    }

    // 3. User global: ~/.claude/CLAUDE.md
    if let Some(home) = dirs::home_dir() {
        let user_md = home.join(".claude").join("CLAUDE.md");
        if let Some(mf) = try_load(&user_md, MemoryScope::UserGlobal) {
            files.push(mf);
        }
    }

    debug!(count = files.len(), "loaded memory files");
    files
}

fn try_load(path: &Path, scope: MemoryScope) -> Option<MemoryFile> {
    if path.exists() {
        match std::fs::read_to_string(path) {
            Ok(content) => Some(MemoryFile {
                path: path.to_path_buf(),
                content,
                scope,
            }),
            Err(e) => {
                tracing::warn!(?path, %e, "failed to read memory file");
                None
            }
        }
    } else {
        None
    }
}

/// Concatenate all memory file contents into a single system prompt section.
pub fn build_memory_prompt(files: &[MemoryFile]) -> String {
    if files.is_empty() {
        return String::new();
    }

    let mut parts = Vec::new();
    for file in files {
        let scope_label = match file.scope {
            MemoryScope::Project => "project instructions",
            MemoryScope::ProjectConfig => "project config instructions",
            MemoryScope::UserGlobal => "user global instructions",
        };
        parts.push(format!(
            "Contents of {} ({scope_label}):\n\n{}",
            file.path.display(),
            file.content,
        ));
    }
    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Filter to only project-scoped files (ignore user global in tests)
    fn project_only(files: Vec<MemoryFile>) -> Vec<MemoryFile> {
        files.into_iter().filter(|f| f.scope != MemoryScope::UserGlobal).collect()
    }

    #[test]
    fn load_from_project_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "# Project rules\nBe helpful.").unwrap();

        let files = project_only(load_memory_files(dir.path()));
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].scope, MemoryScope::Project);
        assert!(files[0].content.contains("Be helpful"));
    }

    #[test]
    fn load_from_project_config() {
        let dir = tempfile::tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("CLAUDE.md"), "Config rules").unwrap();

        let files = project_only(load_memory_files(dir.path()));
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].scope, MemoryScope::ProjectConfig);
    }

    #[test]
    fn load_both_project_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "Root rules").unwrap();
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir(&claude_dir).unwrap();
        std::fs::write(claude_dir.join("CLAUDE.md"), "Config rules").unwrap();

        let files = project_only(load_memory_files(dir.path()));
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].scope, MemoryScope::Project);
        assert_eq!(files[1].scope, MemoryScope::ProjectConfig);
    }

    #[test]
    fn empty_dir_no_project_files() {
        let dir = tempfile::tempdir().unwrap();
        let files = project_only(load_memory_files(dir.path()));
        assert!(files.is_empty());
    }

    #[test]
    fn build_prompt_from_files() {
        let files = vec![
            MemoryFile {
                path: PathBuf::from("/project/CLAUDE.md"),
                content: "Rule 1".into(),
                scope: MemoryScope::Project,
            },
            MemoryFile {
                path: PathBuf::from("/home/.claude/CLAUDE.md"),
                content: "Rule 2".into(),
                scope: MemoryScope::UserGlobal,
            },
        ];

        let prompt = build_memory_prompt(&files);
        assert!(prompt.contains("Rule 1"));
        assert!(prompt.contains("Rule 2"));
        assert!(prompt.contains("project instructions"));
        assert!(prompt.contains("user global instructions"));
    }
}
