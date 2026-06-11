# Technical Architecture

## Overview

Codeplag-rs is a multi-dimensional code plagiarism analyzer that generates 9 distinct fingerprint types from source code and combines them via weighted ensemble scoring to produce similarity results.

## Pipeline

```
Source Code
    │
    ├──► fingerprint/winnowing ──► Winnowing hashes (text-level)
    │       · strip_comments()
    │       · normalize_whitespace()
    │       · tokenize() → 6 TokenKinds
    │       · compute_k_gram_hashes() (identifiers → 0xFF placeholder)
    │       · winnow() → sparse fingerprint selection
    │
    └──► fingerprint/ast ──► Structural hashes (AST-level)
            · generate_ast_hashes() → structural node hashing
            · generate_semantic_ast_hashes() → for↔while, match↔if normalization
            · generate_cfg_hashes() → control flow graph hashing
            · generate_call_graph_hashes() → caller→callee edges
            · generate_def_use_hashes() → variable def→use data flow
            · generate_statement_hashes() → statement trigrams
            · generate_bag_ast_hashes() → order-independent hashing
            · compute_token_frequency() → 6D cosine similarity
            │
            ▼
    engine::SimilarityEngine
            │
            · Weighted ensemble scoring
            · Threshold filtering
            · Chunk matching (vote-based offset alignment)
            · FingerprintCache (SHA-256 disk cache)
            │
            ▼
    SimilarityResult / ProjectResult / FunctionMatch
```

## Core Data Structures

### CodeFingerprint

```rust
struct CodeFingerprint {
    winnowing_hashes: Vec<u32>,         // Winnowing sparse fingerprints
    fingerprint_lines: Vec<(u32, usize)>, // Fingerprints with line numbers
    all_kgraph_lines: Vec<(u32, usize)>,  // All k-gram hashes (dense, for chunks)
    ast_hashes: Vec<u64>,               // AST structural + semantic normalization
    token_freq: Vec<f64>,               // 6D token frequency vector
    cfg_hashes: Vec<u64>,               // Control flow graph hashes
    bag_ast_hashes: Vec<u64>,           // Order-independent AST hashes
    call_graph_hashes: Vec<u64>,        // Call graph edge hashes
    def_use_hashes: Vec<u64>,           // Def-use graph edge hashes
    stmt_hashes: Vec<u64>,              // Statement trigram hashes
}
```

## Scoring Formulas

### File-level (with AST)

```
score = 0.35 × Winnowing + 0.20 × AST-structural
      + 0.10 × Bag-AST      + 0.05 × Token-cosine
      + 0.10 × CFG           + 0.05 × CallGraph
      + 0.05 × DefUse        + 0.10 × StmtTrigram
```

### Project-level (Coverage-aware)

```
project_score = Σ(best_file_matches) / max(|ProjectA|, |ProjectB|)
```

The denominator uses the size of the larger project, penalizing projects that only partially match.

## Supported Languages

| Language | Extension | Tree-sitter Grammar |
|----------|-----------|-------------------|
| Rust     | .rs       | tree-sitter-rust 0.23 |
| Python   | .py       | tree-sitter-python 0.23 |
| JavaScript | .js    | tree-sitter-javascript 0.23 |
| TypeScript | .ts    | tree-sitter-javascript 0.23 |
| Go       | .go       | tree-sitter-go 0.23 |
| C        | .c, .h    | tree-sitter-c 0.23 |
| C++      | .cpp, .cc, .cxx, .hpp | tree-sitter-cpp 0.23 |
| Java     | .java     | tree-sitter-java 0.23 |

Cross-language files are never compared against each other.
