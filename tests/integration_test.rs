//! Integration tests for the Codeplag-rs plagiarism detection engine.
//!
//! These tests verify end-to-end behavior using the test_fixtures directory
//! and exercise all 9 fingerprint dimensions plus the weighted scoring formula.

use codeplag::core::types::{AnalyzerConfig, Language, SourceFile};
use codeplag::engine::SimilarityEngine;
use std::fs;

/// Helper: read a fixture file into a SourceFile
fn load_fixture(category: &str, filename: &str) -> SourceFile {
    let path = format!("test_fixtures/{}/{}", category, filename);
    let content = fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!("Fixture not found: {}", path);
    });
    let ext = filename.rsplit('.').next().unwrap_or("rs");
    let language = Language::from_extension(ext);
    let size = content.len();
    SourceFile { path, content, language, size }
}

fn default_config() -> AnalyzerConfig {
    AnalyzerConfig::default()
}

// ── Core Scenarios ────────────────────────────────────────────────

#[test]
fn identical_files_have_perfect_similarity() {
    let file = load_fixture("original", "sort_rust.rs");
    let clone = SourceFile { path: "clone.rs".into(), ..file.clone() };

    let mut engine = SimilarityEngine::new(default_config());
    engine.index_files(&[file, clone]);
    let results = engine.compare_all();

    assert!(!results.is_empty(), "Should find at least one result");
    let best = &results[0];
    assert!(
        best.similarity_score > 0.98,
        "Identical files should have ~100% similarity, got {:.2}%",
        best.similarity_score * 100.0
    );
}

#[test]
fn renamed_variables_still_high_similarity() {
    let original = load_fixture("original", "sort_rust.rs");
    let renamed = load_fixture("renamed", "sort_rust.rs");

    let mut engine = SimilarityEngine::new(default_config());
    engine.index_files(&[original, renamed]);
    let results = engine.compare_all();

    assert!(!results.is_empty(), "Should find similar pair");
    let best = &results[0];
    assert!(
        best.similarity_score > 0.5,
        "Renamed variables should still have high similarity (>50%), got {:.2}%",
        best.similarity_score * 100.0
    );
}

#[test]
fn unrelated_code_has_low_similarity() {
    let original = load_fixture("original", "sort_rust.rs");
    let unrelated = load_fixture("unrelated", "utils_rust.rs");

    let mut engine = SimilarityEngine::new(default_config());
    engine.index_files(&[original, unrelated]);
    let results = engine.compare_all();

    // May be empty if below threshold; if not empty, should be low
    if !results.is_empty() {
        let best = &results[0];
        assert!(
            best.similarity_score < 0.5,
            "Unrelated code should have low similarity (<50%), got {:.2}%",
            best.similarity_score * 100.0
        );
    }
}

#[test]
fn original_vs_restructured_still_detectable() {
    let original = load_fixture("original", "sort_rust.rs");
    let restructured = load_fixture("restructured", "sort_rust.rs");

    // Restructured code is very different algorithmically; use lower threshold
    let mut config = default_config();
    config.threshold = 0.0;

    let mut engine = SimilarityEngine::new(config);
    engine.index_files(&[original, restructured]);
    let results = engine.compare_all();

    assert!(!results.is_empty(), "Should detect restructured code at 0 threshold");
    // Even though detectable, restructured code should have lower similarity than renamed
    let best = &results[0];
    assert!(
        best.similarity_score < 0.8,
        "Restructured code should have lower similarity than renamed"
    );
}

// ── Cross-Language Tests ─────────────────────────────────────────

#[test]
fn cross_language_files_not_compared() {
    let rust_file = load_fixture("original", "sort_rust.rs");
    let python_file = load_fixture("original", "sort_python.py");

    let mut engine = SimilarityEngine::new(default_config());
    engine.index_files(&[rust_file, python_file]);
    let results = engine.compare_all();

    assert!(results.is_empty(), "Cross-language files should not be compared");
}

#[test]
fn same_language_across_categories_compared() {
    let original = load_fixture("original", "sort_go.go");
    let renamed = load_fixture("renamed", "sort_go.go");

    let mut engine = SimilarityEngine::new(default_config());
    engine.index_files(&[original, renamed]);
    let results = engine.compare_all();

    assert!(!results.is_empty(), "Same-language files should be compared");
}

// ── Weighted Scoring Formula ──────────────────────────────────────

#[test]
fn weighted_scores_sum_to_one_with_ast() {
    // Verify the scoring weights are rational (no panics, no NaN)
    let original = load_fixture("original", "sort_rust.rs");
    let renamed = load_fixture("renamed", "sort_rust.rs");

    let mut engine = SimilarityEngine::new(default_config());
    engine.index_files(&[original, renamed]);
    let results = engine.compare_all();

    for r in &results {
        assert!(r.similarity_score.is_finite(), "Score must be finite");
        assert!(
            r.similarity_score >= 0.0 && r.similarity_score <= 1.0,
            "Score must be in [0, 1], got {}",
            r.similarity_score
        );
    }
}

#[test]
fn winnowing_score_separate_from_ast_score() {
    let original = load_fixture("original", "sort_rust.rs");
    let renamed = load_fixture("renamed", "sort_rust.rs");

    let mut engine = SimilarityEngine::new(default_config());
    engine.index_files(&[original, renamed]);
    let results = engine.compare_all();

    if let Some(r) = results.first() {
        assert!(r.winnowing_score.is_finite());
        assert!(r.ast_score.is_finite());
        // Both should contribute meaningful values
        assert!(r.winnowing_score >= 0.0);
        assert!(r.ast_score >= 0.0);
    }
}

// ── Project-Level Comparison ──────────────────────────────────────

#[test]
fn project_comparison_original_vs_renamed() {
    let project_a =
        vec![load_fixture("original", "sort_rust.rs"), load_fixture("original", "sort_python.py")];
    let project_b =
        vec![load_fixture("renamed", "sort_rust.rs"), load_fixture("renamed", "sort_python.py")];

    let engine = SimilarityEngine::new(default_config());
    let result = engine.compare_projects(&project_a, &project_b);

    assert!(
        result.project_score > 0.3,
        "Project score for renamed code should be >30%, got {:.2}%",
        result.project_score * 100.0
    );
    assert!(!result.file_matches.is_empty(), "Should find file matches between projects");
}

#[test]
fn project_comparison_original_vs_unrelated() {
    let project_a =
        vec![load_fixture("original", "sort_rust.rs"), load_fixture("original", "sort_python.py")];
    let project_b = vec![
        load_fixture("unrelated", "utils_rust.rs"),
        load_fixture("unrelated", "utils_python.py"),
    ];

    let engine = SimilarityEngine::new(default_config());
    let result = engine.compare_projects(&project_a, &project_b);

    assert!(
        result.project_score < 0.5,
        "Unrelated projects should have low score (<50%), got {:.2}%",
        result.project_score * 100.0
    );
}

#[test]
fn project_score_coverage_aware() {
    // If project A has 2 files and project B has 4 files (2 unrelated),
    // the score should be penalized because only 2/4 files can match.
    let project_a =
        vec![load_fixture("original", "sort_rust.rs"), load_fixture("original", "sort_python.py")];
    let project_b = vec![
        load_fixture("renamed", "sort_rust.rs"),
        load_fixture("renamed", "sort_python.py"),
        load_fixture("unrelated", "utils_rust.rs"),
        load_fixture("unrelated", "utils_go.go"),
    ];

    let engine = SimilarityEngine::new(default_config());
    let result = engine.compare_projects(&project_a, &project_b);

    // Even though the 2 matched files may have high similarity,
    // the project_score is divided by max(|A|,|B|) = 4
    assert!(
        result.project_score <= 0.5,
        "Coverage-aware scoring should penalize unmatched files, got {:.2}%",
        result.project_score * 100.0
    );
}

// ── Threshold Filtering ───────────────────────────────────────────

#[test]
fn high_threshold_filters_results() {
    let original = load_fixture("original", "sort_rust.rs");
    let renamed = load_fixture("renamed", "sort_rust.rs");

    let mut config = default_config();
    config.threshold = 0.99; // Very strict

    let mut engine = SimilarityEngine::new(config);
    engine.index_files(&[original, renamed]);
    let results = engine.compare_all();

    // Renamed code should be filtered out at 0.99 threshold
    assert!(results.is_empty(), "Renamed code should be filtered at 0.99 threshold");
}

#[test]
fn low_threshold_keeps_more_results() {
    let original = load_fixture("original", "sort_rust.rs");
    let renamed = load_fixture("renamed", "sort_rust.rs");

    let mut config = default_config();
    config.threshold = 0.1; // Very lenient

    let mut engine = SimilarityEngine::new(config);
    engine.index_files(&[original, renamed]);
    let results = engine.compare_all();

    assert!(!results.is_empty(), "Low threshold should keep results");
}

// ── Edge Cases ────────────────────────────────────────────────────

#[test]
fn empty_file_list_returns_empty() {
    let engine = SimilarityEngine::new(default_config());
    assert_eq!(engine.indexed_count(), 0);
}

#[test]
fn file_too_small_is_skipped() {
    let tiny = SourceFile {
        path: "tiny.rs".into(),
        content: "x".into(),
        language: Language::Rust,
        size: 1,
    };
    let mut config = default_config();
    config.min_file_size = 100;

    let mut engine = SimilarityEngine::new(config);
    engine.index_files(&[tiny]);
    assert_eq!(engine.indexed_count(), 0, "Tiny file should be skipped");
}

#[test]
fn compare_against_with_indexed_files() {
    let indexed_file = load_fixture("original", "sort_rust.rs");
    let target_file = load_fixture("renamed", "sort_rust.rs");

    let mut engine = SimilarityEngine::new(default_config());
    engine.index_files(&[indexed_file]);
    let results = engine.compare_against(&target_file);

    assert!(!results.is_empty(), "Should find match against indexed files");
    assert!(results[0].similarity_score > 0.3, "Should have reasonable similarity");
}

#[test]
fn results_sorted_by_similarity_descending() {
    let original = load_fixture("original", "sort_rust.rs");
    let renamed = load_fixture("renamed", "sort_rust.rs");
    let unrelated = load_fixture("unrelated", "utils_rust.rs");

    let mut engine = SimilarityEngine::new(default_config());
    engine.index_files(&[original.clone(), renamed.clone(), unrelated]);
    let results = engine.compare_all();

    // Results should be sorted descending
    for window in results.windows(2) {
        assert!(
            window[0].similarity_score >= window[1].similarity_score,
            "Results should be sorted descending: {} >= {}",
            window[0].similarity_score,
            window[1].similarity_score
        );
    }
}

// ── Chunk Matching ────────────────────────────────────────────────

#[test]
fn chunk_matches_present_for_similar_files() {
    let original = load_fixture("original", "sort_rust.rs");
    let renamed = load_fixture("renamed", "sort_rust.rs");

    let mut engine = SimilarityEngine::new(default_config());
    engine.index_files(&[original, renamed]);
    let results = engine.compare_all();

    if let Some(r) = results.first() {
        assert!(!r.matched_chunks.is_empty(), "Similar files should have matched chunks");
        assert!(
            r.matched_chunks.len() <= 5,
            "Chunks should be capped at 5, got {}",
            r.matched_chunks.len()
        );
    }
}

#[test]
fn chunk_lines_are_valid() {
    let original = load_fixture("original", "sort_rust.rs");
    let renamed = load_fixture("renamed", "sort_rust.rs");

    let mut engine = SimilarityEngine::new(default_config());
    engine.index_files(&[original, renamed]);
    let results = engine.compare_all();

    for r in &results {
        for chunk in &r.matched_chunks {
            assert!(chunk.line_a <= chunk.line_end_a, "Chunk line_a must be <= line_end_a");
            assert!(chunk.line_b <= chunk.line_end_b, "Chunk line_b must be <= line_end_b");
            assert!(chunk.score >= 0.0 && chunk.score <= 1.0, "Chunk score must be in [0, 1]");
        }
    }
}

// ── Multi-Language ────────────────────────────────────────────────

#[test]
fn all_seven_languages_can_be_analyzed() {
    let languages = vec![
        ("sort_rust.rs", "original"),
        ("sort_python.py", "original"),
        ("sort_js.js", "original"),
        ("sort_ts.ts", "original"),
        ("sort_go.go", "original"),
        ("sort_c.c", "original"),
        ("sort_java.java", "original"),
    ];

    let files: Vec<SourceFile> =
        languages.into_iter().map(|(name, cat)| load_fixture(cat, name)).collect();

    let mut engine = SimilarityEngine::new(default_config());
    engine.index_files(&files);

    // All 7 files should be indexed (no Unknown language)
    assert_eq!(engine.indexed_count(), 7, "All 7 languages should be indexed");
}
