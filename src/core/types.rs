use serde::{Deserialize, Serialize};

/// Supported programming languages for analysis.
///
/// Each variant maps to a file extension and a corresponding tree-sitter grammar
/// for AST-level structural analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    /// Rust (`.rs`)
    Rust,
    /// Python (`.py`)
    Python,
    /// JavaScript (`.js`)
    JavaScript,
    /// TypeScript (`.ts`)
    TypeScript,
    /// Go (`.go`)
    Go,
    /// C (`.c`, `.h`)
    C,
    /// C++ (`.cpp`, `.cc`, `.cxx`, `.hpp`)
    Cpp,
    /// Java (`.java`)
    Java,
    /// Unknown or unsupported language
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
            Language::Go => Some(tree_sitter_go::LANGUAGE.into()),
            Language::C => Some(tree_sitter_c::LANGUAGE.into()),
            Language::Cpp => Some(tree_sitter_cpp::LANGUAGE.into()),
            Language::Java => Some(tree_sitter_java::LANGUAGE.into()),
            Language::Unknown => None,
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
    /// Note: These are the selected hashes after applying the winnowing algorithm, which are used for efficient similarity comparison.
    pub winnowing_hashes: Vec<u32>,
    /// Winnowing fingerprints with line numbers for chunk matching
    /// Note: This includes the line numbers corresponding to each k-gram hash, which allows for more accurate chunk-level similarity detection and reporting.
    /// need of chunk matching
    pub fingerprint_lines: Vec<(u32, usize)>,
    /// ALL k-gram hashes with line numbers (dense, for accurate chunk matching)
    pub all_kgraph_lines: Vec<(u32, usize)>,
    /// AST subtree structural hashes
    pub ast_hashes: Vec<u64>,
    /// Token frequency vector for cosine similarity
    pub token_freq: Vec<f64>,
    /// CFG (Control Flow Graph) structural hashes
    pub cfg_hashes: Vec<u64>,
    /// Order-independent bag-of-statements AST hashes
    pub bag_ast_hashes: Vec<u64>,
    /// Call graph edge hashes (caller→callee relationships)
    pub call_graph_hashes: Vec<u64>,
    /// Def-use graph hashes (variable definition → use edges, name-abstracted)
    pub def_use_hashes: Vec<u64>,
    /// Statement trigram hashes (three consecutive statement types, order-aware)
    pub stmt_hashes: Vec<u64>,
    /// Token count (for normalization)
    pub token_count: usize,
    /// Language of the file
    pub language: Language,
}

/// A code snippet extracted from a source file (e.g., a function).
/// need for function-level comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSnippet {
    /// Function name
    pub name: String,
    /// Source code of the function
    pub content: String,
    /// Start line in the source file (1-based)
    pub start_line: usize,
    /// End line in the source file (1-based)
    pub end_line: usize,
    /// Language of the function
    pub language: Language,
}

/// Result of comparing two projects (directories)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectResult {
    /// Overall project similarity score [0.0, 1.0]
    pub project_score: f64,
    /// Per-file best matches
    pub file_matches: Vec<ProjectFileMatch>,
}

/// Best match for a single file in project A against project B
/// need for reporting results at file level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFileMatch {
    /// File path in project A
    pub file_a: String,
    /// Best matching file in project B
    pub file_b: String,
    /// Similarity score for this pair
    pub similarity_score: f64,
    /// Winnowing score
    pub winnowing_score: f64,
    /// AST score
    pub ast_score: f64,
}

/// Result of comparing two functions
#[derive(Debug, Clone, Serialize, Deserialize)]
/// This struct represents a matched pair of functions from two different files, along with their similarity scores and line ranges. It is used for function-level comparison when the `--functions` flag is enabled in the CLI.
pub struct FunctionMatch {
    /// Function name in file A
    pub func_a: String,
    /// File path of A
    pub file_a: String,
    /// Line range in file A
    pub lines_a: (usize, usize),
    /// Function name in file B
    pub func_b: String,
    /// File path of B
    pub file_b: String,
    /// Line range in file B
    pub lines_b: (usize, usize),
    /// Similarity score [0.0, 1.0]
    pub similarity_score: f64,
    /// Winnowing-based similarity
    pub winnowing_score: f64,
    /// AST-based similarity
    pub ast_score: f64,
}

/// A matched chunk of code between two files
#[derive(Debug, Clone, Serialize, Deserialize)]
/// matched code chunk between two files, with line ranges and similarity score. This is used for detailed reporting of which specific parts of the code are similar, especially when using winnowing-based chunk matching. The line numbers allow us to highlight the matched code in the original source files when presenting results to the user.
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
/// configure parameters for the plagiarism analyzer, such as k-gram size, window size, similarity threshold, and file size limits. This struct allows us to easily manage and pass around configuration settings throughout the analysis process, and provides default values that can be overridden by command-line arguments or configuration files.
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
/// Default configuration values for the analyzer
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_extension_rust() {
        assert_eq!(Language::from_extension("rs"), Language::Rust);
    }

    #[test]
    fn test_language_from_extension_python() {
        assert_eq!(Language::from_extension("py"), Language::Python);
    }

    #[test]
    fn test_language_from_extension_javascript() {
        assert_eq!(Language::from_extension("js"), Language::JavaScript);
    }

    #[test]
    fn test_language_from_extension_typescript() {
        assert_eq!(Language::from_extension("ts"), Language::TypeScript);
    }

    #[test]
    fn test_language_from_extension_go() {
        assert_eq!(Language::from_extension("go"), Language::Go);
    }

    #[test]
    fn test_language_from_extension_c() {
        assert_eq!(Language::from_extension("c"), Language::C);
    }

    #[test]
    fn test_language_from_extension_cpp() {
        assert_eq!(Language::from_extension("cpp"), Language::Cpp);
        assert_eq!(Language::from_extension("cc"), Language::Cpp);
        assert_eq!(Language::from_extension("cxx"), Language::Cpp);
    }

    #[test]
    fn test_language_from_extension_java() {
        assert_eq!(Language::from_extension("java"), Language::Java);
    }

    #[test]
    fn test_language_from_extension_unknown() {
        assert_eq!(Language::from_extension("txt"), Language::Unknown);
        assert_eq!(Language::from_extension(""), Language::Unknown);
    }

    #[test]
    fn test_tree_sitter_language_rust() {
        assert!(Language::Rust.tree_sitter_language().is_some());
    }

    #[test]
    fn test_tree_sitter_language_unknown() {
        assert!(Language::Unknown.tree_sitter_language().is_none());
    }

    #[test]
    fn test_default_config_values() {
        let config = AnalyzerConfig::default();
        assert_eq!(config.k_gram_size, 5);
        assert_eq!(config.window_size, 4);
        assert!((config.threshold - 0.5).abs() < 1e-10);
        assert_eq!(config.min_file_size, 100);
        assert_eq!(config.max_file_size, 1_000_000);
    }

    #[test]
    fn test_source_file_creation() {
        let sf = SourceFile {
            path: "test.rs".into(),
            content: "fn main() {}".into(),
            language: Language::Rust,
            size: 13,
        };
        assert_eq!(sf.path, "test.rs");
        assert_eq!(sf.language, Language::Rust);
    }

    #[test]
    fn test_function_snippet_creation() {
        let snippet = FunctionSnippet {
            name: "main".into(),
            content: "fn main() {}".into(),
            start_line: 1,
            end_line: 1,
            language: Language::Rust,
        };
        assert_eq!(snippet.name, "main");
        assert_eq!(snippet.start_line, 1);
    }

    #[test]
    fn test_similarity_result_creation() {
        let result = SimilarityResult {
            file_a: "a.rs".into(),
            file_b: "b.rs".into(),
            similarity_score: 0.85,
            winnowing_score: 0.9,
            ast_score: 0.8,
            matched_chunks: vec![],
        };
        assert!(result.similarity_score > 0.0);
        assert!(result.similarity_score <= 1.0);
    }
}
