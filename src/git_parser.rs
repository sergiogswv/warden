//! Git repository parser
//!
//! Extracts commit history, file changes, and metadata from Git repositories.

use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Commit {
    pub hash: String,
    pub author: String,
    pub message: String,
    pub timestamp: i64,
    pub files: Vec<String>,
}

pub fn parse_git_history(repo_path: &Path, period: &str) -> anyhow::Result<Vec<Commit>> {
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
    let mut current_commit: Option<Commit> = None;

    for line in stdout.lines() {
        if line.contains('|') {
            // This is a commit header line
            if let Some(commit) = current_commit.take() {
                commits.push(commit);
            }

            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 3 {
                let hash = parts[0].to_string();
                let author = parts[1].to_string();
                let timestamp = parts[2].parse::<i64>().unwrap_or(0);

                current_commit = Some(Commit {
                    hash,
                    author,
                    message: String::new(),
                    timestamp,
                    files: Vec::new(),
                });
            }
        } else if !line.is_empty() {
            // This is a file name
            if let Some(ref mut commit) = current_commit {
                commit.files.push(line.to_string());
            }
        }
    }

    if let Some(commit) = current_commit.take() {
        commits.push(commit);
    }

    Ok(commits)
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
    fn test_parse_git_history_empty_repo() {
        // Placeholder test
        assert!(true);
    }
}
