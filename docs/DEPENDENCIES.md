# Dependencies

## Runtime Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| clap | 4 | CLI argument parsing with derive macros |
| tokio | 1 | Async runtime for GitHub API requests |
| reqwest | 0.12 | HTTP client for GitHub API |
| serde / serde_json | 1 | Serialization for cache and JSON output |
| sha2 | 0.10 | SHA-256 hashing for fingerprints and cache keys |
| tree-sitter | 0.24 | AST parsing framework |
| tree-sitter-{rust,python,javascript,go,c,cpp,java} | 0.23 | Language-specific grammars |
| log / env_logger | 0.4 / 0.11 | Structured logging |
| thiserror / anyhow | 2 / 1 | Error handling |
| glob | 0.3 | File pattern matching |
| indicatif | 0.17 | Progress bars for batch operations |
| rayon | 1 | Parallel computation for pairwise comparisons |

## Dev Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| assert_cmd | 2 | CLI integration testing |
| predicates | 3 | Output assertion helpers |
| tempfile | 3 | Temporary directories for tests |

## Build Tools

| Tool | Purpose |
|------|---------|
| cargo-tarpaulin | Code coverage reporting |
| clippy | Lint checks |
| rustfmt | Code formatting |
