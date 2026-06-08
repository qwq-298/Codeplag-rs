use anyhow::{Context, Result};
use glob::Pattern;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::core::types::{Language, SourceFile};

/// Fetcher for GitHub repositories
pub struct GitHubFetcher {
    /// Local directory to clone repositories into
    work_dir: PathBuf,
    /// File patterns to include (e.g., "*.rs", "*.py")
    include_patterns: Vec<String>,
    /// File patterns to exclude
    exclude_patterns: Vec<String>,
}

impl GitHubFetcher {
    /// Create a new GitHubFetcher
    pub fn new(work_dir: impl Into<PathBuf>) -> Self {
        Self {
            work_dir: work_dir.into(),
            include_patterns: vec![
                "*.rs".into(), "*.py".into(), "*.js".into(),
                "*.ts".into(), "*.go".into(), "*.c".into(),
                "*.cpp".into(), "*.h".into(), "*.hpp".into(),
                "*.java".into(),
            ],
            exclude_patterns: vec![
                "*test*".into(), "*spec*".into(), "*.min.*".into(),
                "vendor/*".into(), "node_modules/*".into(),
                "target/*".into(), "__pycache__/*".into(),
            ],
        }
    }

    /// Set custom include patterns
    pub fn with_include_patterns(mut self, patterns: Vec<String>) -> Self {
        self.include_patterns = patterns;
        self
    }

    /// Set custom exclude patterns
    pub fn with_exclude_patterns(mut self, patterns: Vec<String>) -> Self {
        self.exclude_patterns = patterns;
        self
    }

    /// Clone a GitHub repository and collect source files
    pub fn fetch_repo(&self, repo_url: &str) -> Result<Vec<SourceFile>> {
        let repo_name = extract_repo_name(repo_url);
        let repo_dir = self.work_dir.join(&repo_name);

        // Clone if not already present
        if !repo_dir.exists() {
            self.clone_repo(repo_url, &repo_dir)?;
        }

        self.collect_files(&repo_dir)
    }

    /// Collect source files from a local directory
    pub fn collect_local(&self, dir: impl AsRef<Path>) -> Result<Vec<SourceFile>> {
        self.collect_files(dir.as_ref())
    }

    /// Clone a repository using git
    fn clone_repo(&self, url: &str, dest: &Path) -> Result<()> {
        log::info!("Cloning {} into {}", url, dest.display());

        let output = Command::new("git")
            .args(["clone", "--depth", "1", url])
            .arg(dest)
            .output()
            .context("Failed to execute git clone")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Git clone failed: {}", stderr);
        }

        Ok(())
    }

    /// Recursively collect source files matching include/exclude patterns.
    /// If `dir` is a file, returns a single-element vec with that file.
    fn collect_files(&self, path: &Path) -> Result<Vec<SourceFile>> {
        if path.is_file() {
            // Use full path as-is so main.rs can re-read the file for chunk display
            let display_path = path.to_string_lossy().to_string();
            return Ok(self.read_file(path, &display_path)
                .into_iter()
                .collect());
        }

        let include_globs: Vec<Pattern> = self.include_patterns
            .iter()
            .filter_map(|p| Pattern::new(p).ok())
            .collect();

        let exclude_globs: Vec<Pattern> = self.exclude_patterns
            .iter()
            .filter_map(|p| Pattern::new(p).ok())
            .collect();

        let mut files = Vec::new();
        self.walk_dir(path, path, &include_globs, &exclude_globs, &mut files)?;
        Ok(files)
    }

    fn walk_dir(
        &self,
        base: &Path,
        current: &Path,
        include_globs: &[Pattern],
        exclude_globs: &[Pattern],
        files: &mut Vec<SourceFile>,
    ) -> Result<()> {
        for entry in std::fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            let rel_path = path.strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            // Check exclude patterns
            if exclude_globs.iter().any(|p| p.matches(&rel_path)) {
                continue;
            }

            if path.is_dir() {
                // Skip hidden directories
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy().starts_with('.') {
                        continue;
                    }
                }
                self.walk_dir(base, &path, include_globs, exclude_globs, files)?;
            } else if path.is_file() {
                // Check include patterns
                if !include_globs.iter().any(|p| p.matches(&rel_path)) {
                    continue;
                }

                if let Some(source_file) = self.read_file(&path, &rel_path) {
                    files.push(source_file);
                }
            }
        }
        Ok(())
    }

    fn read_file(&self, path: &Path, rel_path: &str) -> Option<SourceFile> {
        let mut content = std::fs::read_to_string(path).ok()?;
        // Normalize CRLF → LF so tokenizer line tracking works correctly
        content = content.replace("\r\n", "\n");
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let language = Language::from_extension(ext);
        let size = content.len();

        Some(SourceFile {
            path: rel_path.to_string(),
            content,
            language,
            size,
        })
    }
}

/// Extract repository name from GitHub URL
fn extract_repo_name(url: &str) -> String {
    // Handle formats:
    // https://github.com/user/repo.git
    // https://github.com/user/repo
    // git@github.com:user/repo.git
    let url = url.trim_end_matches(".git");
    let parts: Vec<&str> = url.split('/').collect();
    let last = parts.last().copied().unwrap_or("unknown");

    // For SSH URLs: git@github.com:user/repo
    if last.contains(':') {
        last.split(':').next_back().unwrap_or("unknown").to_string()
    } else {
        last.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_repo_name_https() {
        assert_eq!(
            extract_repo_name("https://github.com/user/repo.git"),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_ssh() {
        assert_eq!(
            extract_repo_name("git@github.com:user/repo.git"),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_no_git() {
        assert_eq!(
            extract_repo_name("https://github.com/user/repo"),
            "repo"
        );
    }
}
