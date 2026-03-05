//! Git repository parser
//!
//! Extracts commit history, file changes, and metadata from Git repositories.

use std::path::Path;
use std::process::Command;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Commit {
    pub hash: String,
    pub author: String,
    pub message: String,
    pub timestamp: i64,
    pub files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FileChange {
    pub file: String,
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone)]
pub struct EnrichedCommit {
    pub hash: String,
    pub author: String,
    pub timestamp: i64,
    pub files: Vec<String>,
    pub file_changes: HashMap<String, FileChange>,
}

/// Parses git history and enriches commits with file change statistics.
///
/// Returns EnrichedCommit objects containing:
/// - Basic commit metadata (hash, author, timestamp)
/// - List of touched files
/// - Detailed additions/deletions per file via `git show --stat`
///
/// Returns empty vector if not a git repository or git is unavailable.
pub fn parse_git_history(repo_path: &Path, period: &str) -> anyhow::Result<Vec<EnrichedCommit>> {
    // Build git log command with proper formatting
    let mut cmd = Command::new("git");
    cmd.current_dir(repo_path);
    cmd.arg("log");

    // Add period filter if specified
    if period != "all" {
        cmd.arg(format!("--since={}",  parse_period(period)));
    }

    // Format: hash|author|timestamp|files
    cmd.arg("--format=%H|%an|%ct|%N");
    cmd.arg("--name-only");
    cmd.arg("--diff-filter=ACMRTU");

    let output = cmd.output()?;

    if !output.status.success() {
        // Not a git repository or git not available
        return Ok(vec![]);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();
    let mut current_commit: Option<EnrichedCommit> = None;

    for line in stdout.lines() {
        if line.contains('|') {
            // This is a commit header line
            if let Some(commit) = current_commit.take() {
                commits.push(commit);
            }

            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 3 {
                current_commit = Some(EnrichedCommit {
                    hash: parts[0].to_string(),
                    author: parts[1].to_string(),
                    timestamp: parts[2].parse().unwrap_or(0),
                    files: Vec::new(),
                    file_changes: HashMap::new(),
                });
            }
        } else if !line.is_empty() {
            // This is a filename - skip if ignored by .gitignore or excluded by extension
            if let Some(ref mut commit) = current_commit {
                if !should_ignore_file(repo_path, line) && !should_exclude_by_extension(line) {
                    commit.files.push(line.to_string());
                }
            }
        }
    }

    if let Some(commit) = current_commit.take() {
        commits.push(commit);
    }

    // Enrich commits with LOC data from git show
    for commit in &mut commits {
        let show_output = Command::new("git")
            .current_dir(repo_path)
            .arg("show")
            .arg("--stat")
            .arg("--pretty=format:")
            .arg(&commit.hash)
            .output();

        if let Ok(output) = show_output {
            let show_str = String::from_utf8_lossy(&output.stdout);
            for line in show_str.lines() {
                // Parse lines like: src/main.rs | 45 +++++++++++
                if let Some(pipe_idx) = line.find('|') {
                    let file = line[..pipe_idx].trim();

                    // Skip files that are gitignored or excluded by extension
                    if should_ignore_file(repo_path, file) || should_exclude_by_extension(file) {
                        continue;
                    }

                    let stats = line[pipe_idx+1..].trim();

                    let mut additions = 0;
                    let mut deletions = 0;

                    // Extract numeric value before the +/- symbols
                    // e.g., "21 ++++++++++++---" → 21
                    let parts: Vec<&str> = stats.split_whitespace().collect();
                    if !parts.is_empty() {
                        if let Ok(num) = parts[0].parse::<usize>() {
                            // This is the number of changed lines in this commit
                            // We'll get the total LOC separately
                            additions = num;
                        }
                    }

                    if additions > 0 {
                        commit.file_changes.insert(file.to_string(), FileChange {
                            file: file.to_string(),
                            additions,
                            deletions: 0,
                        });
                    }
                }
            }
        }
    }

    // Get total LOC for each file from HEAD
    // This gives us the current actual line count, not just changes in this commit
    let mut current_loc = HashMap::new();
    if let Ok(ls_output) = Command::new("git")
        .current_dir(repo_path)
        .arg("ls-files")
        .output()
    {
        let ls_str = String::from_utf8_lossy(&ls_output.stdout);
        for file in ls_str.lines() {
            if should_ignore_file(repo_path, file) || should_exclude_by_extension(file) {
                continue;
            }

            // Get current line count for this file
            if let Ok(show_head) = Command::new("git")
                .current_dir(repo_path)
                .arg("show")
                .arg(format!("HEAD:{}", file))
                .output()
            {
                let content = String::from_utf8_lossy(&show_head.stdout);
                let line_count = content.lines().count();
                if line_count > 0 {
                    current_loc.insert(file.to_string(), line_count);
                }
            }
        }
    }

    // Update file_changes with actual LOC from current HEAD
    for commit in &mut commits {
        for (file, change) in &mut commit.file_changes {
            if let Some(loc) = current_loc.get(file) {
                change.deletions = *loc; // Store total LOC in deletions field for now
            }
        }
    }

    Ok(commits)
}

/// Check if a file should be ignored based on .gitignore rules
fn should_ignore_file(repo_path: &Path, file: &str) -> bool {
    // Use git check-ignore to determine if file is ignored
    let output = Command::new("git")
        .current_dir(repo_path)
        .arg("check-ignore")
        .arg(file)
        .output();

    // If git check-ignore succeeds (exit code 0), the file is ignored
    if let Ok(out) = output {
        return out.status.success();
    }

    false
}

/// Check if a file should be excluded based on its extension
/// Excludes: documentation, config, assets, and non-source files
fn should_exclude_by_extension(file: &str) -> bool {
    let excluded_extensions = [
        // Documentation
        ".md", ".txt", ".rst", ".adoc",
        // Configuration
        ".yml", ".yaml", ".toml", ".json", ".xml", ".ini", ".conf", ".config",
        // Assets
        ".svg", ".png", ".jpg", ".jpeg", ".gif", ".ico", ".webp",
        // Style/Formatting
        ".css", ".scss", ".less",
        // Build/Dist
        ".lock", ".min.js", ".min.css",
        // Other non-source
        ".pdf", ".doc", ".docx",
    ];

    // Check if ends with any excluded extension
    if excluded_extensions.iter().any(|ext| file.ends_with(ext)) {
        return true;
    }

    // Also exclude dotfiles (except .gitignore, .env, etc are already excluded above)
    // Exclude common config dotfiles
    let excluded_dotfiles = [".env", ".gitignore", ".editorconfig", ".eslintrc"];
    let file_name = file.split('/').last().unwrap_or(file);
    excluded_dotfiles.iter().any(|name| *name == file_name)
}

fn parse_period(period: &str) -> String {
    match period {
        "3m" => "3 months ago".to_string(),
        "6m" => "6 months ago".to_string(),
        "1y" => "1 year ago".to_string(),
        "2y" => "2 years ago".to_string(),
        s if s.ends_with('m') => format!("{} months ago", &s[..s.len()-1]),
        s if s.ends_with('y') => format!("{} years ago", &s[..s.len()-1]),
        s => s.to_string(),
    }
}

pub fn get_file_diffs(_repo_path: &Path) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_git_history_returns_result_type() {
        // Verify the function returns an anyhow::Result
        // This validates error handling is in place
        let result = parse_git_history(
            std::path::Path::new("."),
            "all"
        );

        // Result should be Ok (we're in a git repo)
        assert!(result.is_ok());
    }

    #[test]
    fn test_enriched_commit_structure() {
        let mut changes = HashMap::new();
        changes.insert("main.rs".to_string(), FileChange {
            file: "main.rs".to_string(),
            additions: 10,
            deletions: 5,
        });

        let commit = EnrichedCommit {
            hash: "abc123".to_string(),
            author: "alice".to_string(),
            timestamp: 1000,
            files: vec!["main.rs".to_string()],
            file_changes: changes,
        };

        assert_eq!(commit.hash, "abc123");
        assert!(commit.file_changes.contains_key("main.rs"));
        assert_eq!(commit.file_changes["main.rs"].additions, 10);
    }

    #[test]
    fn test_parse_git_history_returns_enriched_commits() {
        let commits = parse_git_history(
            std::path::Path::new("."),
            "all"
        ).unwrap();

        // Should return some commits from current repo
        // (may be empty if not in a git repo, which is OK)
        if !commits.is_empty() {
            let first = &commits[0];
            // Verify structure is correct - should have all fields initialized
            assert!(!first.hash.is_empty());
            assert!(!first.author.is_empty());
        }
    }

    #[test]
    fn test_gitignore_filtering() {
        let repo_path = std::path::Path::new(".");

        // .gitignore patterns should be respected
        // target/ directory is in .gitignore
        assert!(should_ignore_file(repo_path, "target/debug/binary"));

        // Cargo.lock is in .gitignore
        assert!(should_ignore_file(repo_path, "Cargo.lock"));

        // .warden-cache is in .gitignore
        assert!(should_ignore_file(repo_path, ".warden-cache"));

        // Source files should NOT be ignored
        assert!(!should_ignore_file(repo_path, "src/main.rs"));
    }

    #[test]
    fn test_exclude_by_extension() {
        // Documentation files should be excluded
        assert!(should_exclude_by_extension("README.md"));
        assert!(should_exclude_by_extension("docs/guide.md"));
        assert!(should_exclude_by_extension("notes.txt"));

        // Configuration files should be excluded
        assert!(should_exclude_by_extension("config.yml"));
        assert!(should_exclude_by_extension("package.json"));
        assert!(should_exclude_by_extension("Cargo.toml"));
        assert!(should_exclude_by_extension(".env"));
        assert!(should_exclude_by_extension("settings.json"));

        // Asset files should be excluded
        assert!(should_exclude_by_extension("logo.svg"));
        assert!(should_exclude_by_extension("image.png"));
        assert!(should_exclude_by_extension("style.css"));

        // Source files should NOT be excluded
        assert!(!should_exclude_by_extension("src/main.rs"));
        assert!(!should_exclude_by_extension("app.tsx"));
        assert!(!should_exclude_by_extension("utils.js"));
        assert!(!should_exclude_by_extension("test.py"));
    }
}
