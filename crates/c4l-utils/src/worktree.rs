//! Git worktree management.
//!
//! Maps from: leak-claude-code/src/tools/EnterWorktreeTool/ + Superpowers using-git-worktrees skill

use anyhow::{Context, Result};
use c4l_types::WorktreeInfo;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::debug;

/// Default directory for worktrees.
const WORKTREE_DIR: &str = ".worktrees";

/// Create a new git worktree for isolated work.
///
/// Uses `git worktree add` (more reliable than libgit2 worktree API).
pub fn create_worktree(
    repo_path: &Path,
    branch_name: &str,
    base_dir: Option<&Path>,
) -> Result<WorktreeInfo> {
    let worktree_base = base_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_path.join(WORKTREE_DIR));

    std::fs::create_dir_all(&worktree_base)?;
    let worktree_path = worktree_base.join(branch_name);

    // Get current branch name for base_branch
    let base_branch = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(repo_path)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "HEAD".into());

    debug!(?worktree_path, branch_name, base = %base_branch, "creating worktree");

    let output = Command::new("git")
        .args(["worktree", "add", "-b", branch_name])
        .arg(&worktree_path)
        .current_dir(repo_path)
        .output()
        .context("failed to run git worktree add")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git worktree add failed: {stderr}");
    }

    Ok(WorktreeInfo {
        path: worktree_path,
        branch: branch_name.into(),
        base_branch,
    })
}

/// Remove a worktree.
pub fn remove_worktree(repo_path: &Path, worktree_path: &Path) -> Result<()> {
    debug!(?worktree_path, "removing worktree");

    let output = Command::new("git")
        .args(["worktree", "remove", "--force"])
        .arg(worktree_path)
        .current_dir(repo_path)
        .output()
        .context("failed to run git worktree remove")?;

    if !output.status.success() {
        // Fallback: just delete the directory
        if worktree_path.exists() {
            std::fs::remove_dir_all(worktree_path)?;
        }
        // Prune stale worktree references
        Command::new("git")
            .args(["worktree", "prune"])
            .current_dir(repo_path)
            .output()
            .ok();
    }

    Ok(())
}

/// Check if the worktree directory is in .gitignore.
pub fn is_gitignored(repo_path: &Path, dir_name: &str) -> bool {
    let gitignore = repo_path.join(".gitignore");
    if let Ok(content) = std::fs::read_to_string(&gitignore) {
        content.lines().any(|line| {
            let line = line.trim();
            line == dir_name || line == format!("{dir_name}/")
        })
    } else {
        false
    }
}

/// Ensure the worktree directory is gitignored. Add to .gitignore if not.
pub fn ensure_gitignored(repo_path: &Path, dir_name: &str) -> Result<()> {
    if is_gitignored(repo_path, dir_name) {
        return Ok(());
    }

    let gitignore = repo_path.join(".gitignore");
    let mut content = std::fs::read_to_string(&gitignore).unwrap_or_default();

    if !content.ends_with('\n') && !content.is_empty() {
        content.push('\n');
    }
    content.push_str(&format!("{dir_name}/\n"));

    std::fs::write(&gitignore, content)?;
    debug!(dir_name, "added worktree directory to .gitignore");

    Ok(())
}

/// List existing worktrees via `git worktree list`.
pub fn list_worktrees(repo_path: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .context("failed to run git worktree list")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let paths: Vec<String> = stdout
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(String::from)
        .collect();

    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gitignore_check() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".gitignore"), "target/\n.worktrees/\n").unwrap();

        assert!(is_gitignored(dir.path(), ".worktrees"));
        assert!(!is_gitignored(dir.path(), "src"));
    }

    #[test]
    fn ensure_gitignored_adds_entry() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".gitignore"), "target/\n").unwrap();

        ensure_gitignored(dir.path(), ".worktrees").unwrap();

        let content = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(content.contains(".worktrees/"));
    }

    #[test]
    fn ensure_gitignored_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".gitignore"), ".worktrees/\n").unwrap();

        ensure_gitignored(dir.path(), ".worktrees").unwrap();

        let content = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert_eq!(content.matches(".worktrees").count(), 1);
    }

    #[test]
    fn no_gitignore_creates_one() {
        let dir = tempfile::tempdir().unwrap();
        ensure_gitignored(dir.path(), ".worktrees").unwrap();

        let content = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(content.contains(".worktrees/"));
    }
}
