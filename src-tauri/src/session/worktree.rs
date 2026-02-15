use std::path::{Path, PathBuf};
use std::process::Command;
use std::{collections::HashSet, fs};

#[derive(Debug, Clone)]
pub struct WorktreeEntry {
    pub path: PathBuf,
    pub prunable: bool,
}

#[derive(Debug, Clone)]
pub struct WorktreeService {
    repo_root: PathBuf,
    worktrees_root: PathBuf,
}

impl WorktreeService {
    pub fn from_working_dir(working_dir: impl AsRef<Path>) -> Result<Self, String> {
        let output = Command::new("git")
            .arg("rev-parse")
            .arg("--show-toplevel")
            .current_dir(working_dir.as_ref())
            .output()
            .map_err(|e| format!("Failed to resolve git repository root: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "Working directory is not inside a git repository: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }

        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(Self::new(root))
    }

    pub fn new(repo_root: impl AsRef<Path>) -> Self {
        let repo_root = repo_root.as_ref().to_path_buf();
        let worktrees_root = repo_root.join(".lulu").join("worktrees");
        Self { repo_root, worktrees_root }
    }

    pub fn create_worktree(&self, session_id: &str) -> Result<PathBuf, String> {
        std::fs::create_dir_all(&self.worktrees_root)
            .map_err(|e| format!("Failed to create worktrees root: {}", e))?;

        let worktree_path = self.worktrees_root.join(session_id);

        if worktree_path.exists() {
            self.remove_worktree_at_path(&worktree_path, true)?;
        }

        let output = Command::new("git")
            .arg("worktree")
            .arg("add")
            .arg("--detach")
            .arg(&worktree_path)
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| format!("Failed to run git worktree add: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "git worktree add failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }

        Ok(worktree_path)
    }

    pub fn remove_worktree_for_session(&self, session_id: &str) -> Result<(), String> {
        let path = self.worktrees_root.join(session_id);
        self.remove_worktree_at_path(&path, true)
    }

    pub fn remove_worktree_at_path(&self, path: &Path, force: bool) -> Result<(), String> {
        if !path.exists() {
            return Ok(());
        }

        let mut cmd = Command::new("git");
        cmd.arg("worktree").arg("remove");
        if force {
            cmd.arg("--force");
        }
        let output = cmd
            .arg(path)
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| format!("Failed to run git worktree remove: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "git worktree remove failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }

        Ok(())
    }

    pub fn prune_worktrees(&self) -> Result<(), String> {
        let output = Command::new("git")
            .arg("worktree")
            .arg("prune")
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| format!("Failed to run git worktree prune: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "git worktree prune failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }

        Ok(())
    }

    pub fn list_worktrees(&self) -> Result<Vec<WorktreeEntry>, String> {
        let output = Command::new("git")
            .arg("worktree")
            .arg("list")
            .arg("--porcelain")
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| format!("Failed to run git worktree list: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "git worktree list failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut entries = Vec::new();
        let mut current_path: Option<PathBuf> = None;
        let mut current_prunable = false;

        for line in stdout.lines() {
            if let Some(path) = line.strip_prefix("worktree ") {
                if let Some(existing_path) = current_path.take() {
                    entries.push(WorktreeEntry {
                        path: existing_path,
                        prunable: current_prunable,
                    });
                }
                current_path = Some(PathBuf::from(path));
                current_prunable = false;
            } else if line.starts_with("prunable") {
                current_prunable = true;
            }
        }

        if let Some(existing_path) = current_path {
            entries.push(WorktreeEntry {
                path: existing_path,
                prunable: current_prunable,
            });
        }

        Ok(entries)
    }

    pub fn reconcile_managed_worktrees(&self, expected_paths: &[PathBuf]) -> Result<(), String> {
        let expected: HashSet<PathBuf> = expected_paths.iter().cloned().collect();
        let worktrees = self.list_worktrees()?;

        for entry in worktrees {
            if !entry.path.starts_with(&self.worktrees_root) {
                continue;
            }

            let is_expected = expected.contains(&entry.path);
            if is_expected && !entry.prunable && entry.path.exists() {
                continue;
            }

            if entry.path.exists() {
                let _ = fs::remove_dir_all(&entry.path);
            }
        }

        self.prune_worktrees()
    }

    pub fn worktrees_root(&self) -> &Path {
        &self.worktrees_root
    }

    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }
}
