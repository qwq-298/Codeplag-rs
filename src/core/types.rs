use serde::{Deserialize, Serialize};

/// Supported programming languages for analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    C,
    Cpp,
    Java,
    Unknown,
}

impl Language {
    /// Detect language from file extension
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "rs" => Language::Rust,
            "py" => Language::Python,
            "js" => Language::JavaScript,
            "ts" => Language::TypeScript,
            "go" => Language::Go,
            "c" => Language::C,
            "cpp" | "cc" | "cxx" => Language::Cpp,
            "java" => Language::Java,
            _ => Language::Unknown,
        }
    }

    /// Get the tree-sitter language for this language
    pub fn tree_sitter_language(&self) -> Option<tree_sitter::Language> {
        match self {
            Language::Rust => Some(tree_sitter_rust::LANGUAGE.into()),
            Language::Python => Some(tree_sitter_python::LANGUAGE.into()),
            Language::JavaScript | Language::TypeScript => {
                Some(tree_sitter_javascript::LANGUAGE.into())
            }
            _ => None,
        }
    }
}

/// A code file to be analyzed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFile {
    /// Relative path within the repository
    pub path: String,
    /// Raw source code content
    pub content: String,
    /// Detected programming language
    pub language: Language,
    /// File size in bytes
    pub size: usize,
}

/// A fingerprint generated from source code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeFingerprint {
    /// Source file path
    pub file_path: String,
    /// Winnowing fingerprints (k-gram hashes)
    pub winnowing_hashes: Vec<u32>,
    /// AST subtree structural hashes
    pub ast_hashes: Vec<u64>,
    /// Token count (for normalization)
    pub token_count: usize,
    /// Language of the file
    pub language: Language,
}

/// A matched chunk of code between two files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMatch {
    /// Start line in file A
    pub line_a: usize,
    /// End line in file A
    pub line_end_a: usize,
    /// Start line in file B
    pub line_b: usize,
    /// End line in file B
    pub line_end_b: usize,
    /// Similarity score of this chunk
    pub score: f64,
}

/// Result of comparing two files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityResult {
    /// First file path
    pub file_a: String,
    /// Second file path
    pub file_b: String,
    /// Overall similarity score [0.0, 1.0]
    pub similarity_score: f64,
    /// Winnowing-based similarity
    pub winnowing_score: f64,
    /// AST-based similarity
    pub ast_score: f64,
    /// Matched code chunks
    pub matched_chunks: Vec<ChunkMatch>,
}

/// Configuration for the plagiarism analyzer
#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    /// k-gram size for winnowing (default: 5)
    pub k_gram_size: usize,
    /// Sliding window size for winnowing (default: 4)
    pub window_size: usize,
    /// Minimum similarity threshold for reporting [0.0, 1.0]
    pub threshold: f64,
    /// Minimum file size in bytes to analyze
    pub min_file_size: usize,
    /// Maximum file size in bytes to analyze
    pub max_file_size: usize,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            k_gram_size: 5,
            window_size: 4,
            threshold: 0.5,
            min_file_size: 100,
            max_file_size: 1_000_000, // 1MB
        }
    }
}
