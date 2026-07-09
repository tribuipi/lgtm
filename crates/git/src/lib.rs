//! Local git diffs via the `git` CLI: "the PR I'd open from here" — everything
//! since the merge-base with the default remote head (committed + staged +
//! unstaged + untracked), as one unified patch for diff-core to parse.

use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// How many untracked files to inline into the patch before giving up.
const MAX_UNTRACKED_FILES: usize = 200;

#[derive(Debug, Clone)]
pub struct LocalSource {
    pub repo_root: PathBuf,
    pub branch: String,
    /// Human name of the diff base: "origin/main"-style remote head when one
    /// exists and shares history with HEAD, otherwise "HEAD" (working-tree-only
    /// diff).
    pub base_label: String,
}

/// Resolve a path inside a git repo to its root, current branch, and diff base.
pub fn resolve_local(path: &Path) -> Result<LocalSource> {
    let repo_root = PathBuf::from(
        git(path, &["rev-parse", "--show-toplevel"])
            .with_context(|| format!("{} is not inside a git repository", path.display()))?
            .trim(),
    );
    let branch = git(&repo_root, &["rev-parse", "--abbrev-ref", "HEAD"])?
        .trim()
        .to_string();

    // Default remote head: prefer the recorded origin/HEAD symref, then the
    // conventional names. A candidate only counts if it shares a merge-base
    // with HEAD; otherwise fall back to a working-tree-only diff against HEAD.
    let mut candidates = Vec::new();
    if let Ok(symref) = git(&repo_root, &["symbolic-ref", "refs/remotes/origin/HEAD"]) {
        if let Some(name) = symref.trim().strip_prefix("refs/remotes/") {
            candidates.push(name.to_string());
        }
    }
    candidates.push("origin/main".to_string());
    candidates.push("origin/master".to_string());
    let base_label = candidates
        .into_iter()
        .find(|cand| git(&repo_root, &["merge-base", "HEAD", cand]).is_ok())
        .unwrap_or_else(|| "HEAD".to_string());

    Ok(LocalSource {
        repo_root,
        branch,
        base_label,
    })
}

/// Unified patch of everything that would go into a PR opened from here:
/// merge-base(HEAD, base)..working-tree (two-dot, so committed + staged +
/// unstaged), plus untracked files appended as added-file diffs.
pub fn diff_patch(src: &LocalSource) -> Result<String> {
    let base = if src.base_label == "HEAD" {
        "HEAD".to_string()
    } else {
        git(&src.repo_root, &["merge-base", "HEAD", &src.base_label])?
            .trim()
            .to_string()
    };
    let mut patch = git(
        &src.repo_root,
        &["diff", "-M", "--no-color", "--no-ext-diff", &base],
    )?;

    let untracked = git(&src.repo_root, &["ls-files", "--others", "--exclude-standard"])?;
    for file in untracked.lines().take(MAX_UNTRACKED_FILES) {
        // `--no-index` against /dev/null renders an untracked file as an
        // added-file diff; it exits 1 when the sides differ, which is success
        // here (0 would mean an empty file — also fine, git emits a header).
        let output = Command::new("git")
            .arg("-C")
            .arg(&src.repo_root)
            .args(["diff", "--no-color", "--no-ext-diff", "--no-index", "--", "/dev/null"])
            .arg(file)
            .output()
            .map_err(|err| anyhow!("failed to run git: {err}"))?;
        if !matches!(output.status.code(), Some(0) | Some(1)) {
            bail!(
                "git diff --no-index /dev/null {file} failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        patch.push_str(&String::from_utf8_lossy(&output.stdout));
    }
    Ok(patch)
}

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|err| anyhow!("failed to run git (is git installed?): {err}"))?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use diff_core::FileStatus;
    use std::fs;

    fn run(dir: &Path, args: &[&str]) {
        let output = Command::new(args[0])
            .args(&args[1..])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn init_repo(dir: &Path) {
        run(dir, &["git", "init", "-b", "main"]);
        run(dir, &["git", "config", "user.email", "test@example.com"]);
        run(dir, &["git", "config", "user.name", "Test"]);
        run(dir, &["git", "config", "commit.gpgsign", "false"]);
    }

    #[test]
    fn no_remote_diffs_working_tree_against_head() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        init_repo(dir);
        fs::write(dir.join("a.rs"), "fn main() {}\n").unwrap();
        run(dir, &["git", "add", "."]);
        run(dir, &["git", "commit", "-m", "init"]);

        // Unstaged edit + untracked text file + untracked binary file.
        fs::write(dir.join("a.rs"), "fn main() { println!(); }\n").unwrap();
        fs::write(dir.join("new.txt"), "hello\n").unwrap();
        fs::write(dir.join("blob.bin"), [0u8, 159, 146, 150]).unwrap();

        let src = resolve_local(dir).unwrap();
        assert_eq!(src.repo_root.canonicalize().unwrap(), dir.canonicalize().unwrap());
        assert_eq!(src.branch, "main");
        assert_eq!(src.base_label, "HEAD");

        let patch = diff_patch(&src).unwrap();
        let diff = diff_core::parse_patch(&patch);
        let by_path: Vec<(&str, FileStatus)> = diff
            .files
            .iter()
            .map(|f| (f.display_path(), f.status))
            .collect();
        assert!(by_path.contains(&("a.rs", FileStatus::Modified)), "{by_path:?}");
        assert!(by_path.contains(&("new.txt", FileStatus::Added)), "{by_path:?}");
        assert!(by_path.contains(&("blob.bin", FileStatus::Binary)), "{by_path:?}");
    }

    #[test]
    fn branch_diffs_against_origin_default_head() {
        let tmp = tempfile::tempdir().unwrap();
        let upstream = tmp.path().join("upstream");
        fs::create_dir(&upstream).unwrap();
        init_repo(&upstream);
        fs::write(upstream.join("lib.rs"), "pub fn one() {}\n").unwrap();
        run(&upstream, &["git", "add", "."]);
        run(&upstream, &["git", "commit", "-m", "init"]);

        let clone = tmp.path().join("clone");
        run(
            tmp.path(),
            &["git", "clone", upstream.to_str().unwrap(), clone.to_str().unwrap()],
        );
        run(&clone, &["git", "config", "user.email", "test@example.com"]);
        run(&clone, &["git", "config", "user.name", "Test"]);
        run(&clone, &["git", "config", "commit.gpgsign", "false"]);
        run(&clone, &["git", "checkout", "-b", "feature"]);
        fs::write(clone.join("lib.rs"), "pub fn one() {}\npub fn two() {}\n").unwrap();
        run(&clone, &["git", "commit", "-am", "add two"]);
        // Plus an uncommitted edit on top: two-dot diff must include it.
        fs::write(
            clone.join("lib.rs"),
            "pub fn one() {}\npub fn two() {}\npub fn three() {}\n",
        )
        .unwrap();

        let src = resolve_local(&clone).unwrap();
        assert_eq!(src.branch, "feature");
        assert_eq!(src.base_label, "origin/main");

        let patch = diff_patch(&src).unwrap();
        let diff = diff_core::parse_patch(&patch);
        assert_eq!(diff.files.len(), 1);
        let file = &diff.files[0];
        assert_eq!(file.display_path(), "lib.rs");
        assert_eq!(file.status, FileStatus::Modified);
        // Committed line + uncommitted line, both present.
        assert_eq!((file.additions, file.deletions), (2, 0));
    }

    #[test]
    fn resolve_rejects_non_repo() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(resolve_local(tmp.path()).is_err());
    }
}
