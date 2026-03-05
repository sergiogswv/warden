//! Caching system for analysis results
//!
//! Stores and retrieves cached metrics to speed up subsequent runs.

use std::fs;
use std::path::Path;

const CACHE_FILENAME: &str = ".warden-cache.json";
const CACHE_MAX_AGE_SECS: u64 = 3600; // 1 hour

pub fn load_cache(repo_path: &Path) -> anyhow::Result<Option<crate::models::AnalysisResult>> {
    let cache_path = repo_path.join(CACHE_FILENAME);

    if !cache_path.exists() {
        return Ok(None);
    }

    if !is_cache_valid(&cache_path, CACHE_MAX_AGE_SECS) {
        return Ok(None);
    }

    let content = fs::read_to_string(&cache_path)?;
    let analysis: crate::models::AnalysisResult = serde_json::from_str(&content)?;

    Ok(Some(analysis))
}

pub fn save_cache(
    repo_path: &Path,
    analysis: &crate::models::AnalysisResult,
) -> anyhow::Result<()> {
    let cache_path = repo_path.join(CACHE_FILENAME);
    let json = serde_json::to_string_pretty(analysis)?;
    fs::write(&cache_path, json)?;

    Ok(())
}

pub fn clear_cache(repo_path: &Path) -> anyhow::Result<()> {
    let cache_path = repo_path.join(CACHE_FILENAME);
    if cache_path.exists() {
        fs::remove_file(&cache_path)?;
    }

    Ok(())
}

fn is_cache_valid(cache_path: &Path, max_age_secs: u64) -> bool {
    if let Ok(metadata) = cache_path.metadata() {
        if let Ok(modified) = metadata.modified() {
            if let Ok(elapsed) = modified.elapsed() {
                return elapsed.as_secs() < max_age_secs;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_path_construction() {
        let repo_path = Path::new(".");
        let cache_path = repo_path.join(CACHE_FILENAME);
        assert!(cache_path.to_string_lossy().contains(".warden-cache.json"));
    }
}
