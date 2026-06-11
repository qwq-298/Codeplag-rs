# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2024-06-10

### Added
- **9-dimensional fingerprint analysis**: Winnowing text fingerprints, AST structural hashing, CFG hashing, call graph analysis, def-use graph analysis, statement trigram hashing, bag-of-statements analysis, semantic normalization (for↔while, match↔if-else), and token frequency cosine similarity.
- **7 programming language support**: Rust, Python, JavaScript, TypeScript, Go, C, C++, Java via tree-sitter grammars.
- **6 CLI commands**: `analyze`, `compare`, `fetch`, `search`, `project`, `batch`.
- **Function-level comparison**: Tree-sitter function extraction and independent per-function fingerprinting.
- **Project-level comparison**: Coverage-aware scoring with cross-project file matching.
- **Chunk matching**: Vote-based offset alignment for locating similar code blocks between files.
- **Fingerprint caching**: SHA-256 content-addressed disk cache to avoid recomputation.
- **Parallel processing**: All pairwise comparisons parallelized via rayon.
- **Multiple output formats**: Text and JSON output for all commands.
- **GitHub integration**: Repository cloning, batch fetching, and code search API support.
- **CI/CD pipeline**: GitHub Actions with build/test (Ubuntu + Windows), clippy/rustfmt static analysis, and tarpaulin code coverage.
- **112 automated tests**: 73 unit tests, 19 CLI integration tests, 20 end-to-end integration tests.
- **Comprehensive documentation**: README with architecture overview, usage guide, and technical deep-dive.

[0.1.0]: https://github.com/qwq-298/Codeplag-rs/releases/tag/v0.1.0
