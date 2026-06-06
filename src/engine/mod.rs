use std::collections::HashMap;
use crate::core::types::{
    AnalyzerConfig, ChunkMatch, CodeFingerprint, SimilarityResult, SourceFile,
};
use crate::fingerprint::winnowing;
use crate::fingerprint::ast;

/// The core similarity analysis engine
pub struct SimilarityEngine {
    config: AnalyzerConfig,
    fingerprints: HashMap<String, CodeFingerprint>,
}

impl SimilarityEngine {
    pub fn new(config: AnalyzerConfig) -> Self {
        Self {
            config,
            fingerprints: HashMap::new(),
        }
    }

    /// Index a set of source files: generate fingerprints for each
    pub fn index_files(&mut self, files: &[SourceFile]) {
        for file in files {
            // Skip files outside size bounds
            if file.size < self.config.min_file_size
                || file.size > self.config.max_file_size
            {
                log::debug!("Skipping {} (size: {})", file.path, file.size);
                continue;
            }

            // Skip unsupported languages
            if file.language == crate::core::types::Language::Unknown {
                log::debug!("Skipping {} (unknown language)", file.path);
                continue;
            }

            log::info!("Indexing: {}", file.path);

            let token_count = file.content.lines().count();

            // Generate winnowing fingerprints
            let winnowing_hashes = winnowing::generate_fingerprints(
                &file.content,
                file.language,
                self.config.k_gram_size,
                self.config.window_size,
            );

            // Generate winnowing fingerprints with line info for chunk matching
            let fingerprint_lines = winnowing::generate_fingerprints_with_lines(
                &file.content,
                file.language,
                self.config.k_gram_size,
                self.config.window_size,
            );

            // Generate AST fingerprints
            let ast_hashes = ast::generate_ast_hashes(&file.content, file.language)
                .unwrap_or_default();

            let fingerprint = CodeFingerprint {
                file_path: file.path.clone(),
                winnowing_hashes,
                fingerprint_lines,
                ast_hashes,
                token_count,
                language: file.language,
            };

            self.fingerprints.insert(file.path.clone(), fingerprint);
        }

        log::info!(
            "Indexed {} files ({} skipped)",
            self.fingerprints.len(),
            files.len() - self.fingerprints.len()
        );
    }

    /// Compare all indexed files against each other
    pub fn compare_all(&self) -> Vec<SimilarityResult> {
        let paths: Vec<&String> = self.fingerprints.keys().collect();
        let mut results = Vec::new();

        for i in 0..paths.len() {
            for j in (i + 1)..paths.len() {
                let fp_a = &self.fingerprints[paths[i]];
                let fp_b = &self.fingerprints[paths[j]];

                // Only compare files of the same language
                if fp_a.language != fp_b.language {
                    continue;
                }

                let winnowing_score = winnowing::jaccard_similarity(
                    &fp_a.winnowing_hashes,
                    &fp_b.winnowing_hashes,
                );

                let ast_score = ast::ast_jaccard_similarity(
                    &fp_a.ast_hashes,
                    &fp_b.ast_hashes,
                );

                // Combined similarity: weighted average
                let similarity_score = if fp_a.ast_hashes.is_empty() {
                    // Fallback to winnowing only if AST parsing failed
                    winnowing_score
                } else {
                    // 40% winnowing + 60% AST (AST is more reliable)
                    0.4 * winnowing_score + 0.6 * ast_score
                };

                if similarity_score >= self.config.threshold {
                    let matched_chunks = find_matching_chunks(fp_a, fp_b);
                    results.push(SimilarityResult {
                        file_a: fp_a.file_path.clone(),
                        file_b: fp_b.file_path.clone(),
                        similarity_score,
                        winnowing_score,
                        ast_score,
                        matched_chunks,
                    });
                }
            }
        }

        // Sort by similarity score descending
        results.sort_by(|a, b| {
            b.similarity_score
                .partial_cmp(&a.similarity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        log::info!("Found {} similar file pairs", results.len());
        results
    }

    /// Compare a single target file against all indexed files
    pub fn compare_against(&self, target: &SourceFile) -> Vec<SimilarityResult> {
        if target.language == crate::core::types::Language::Unknown {
            return Vec::new();
        }

        let fingerprint_lines = winnowing::generate_fingerprints_with_lines(
            &target.content,
            target.language,
            self.config.k_gram_size,
            self.config.window_size,
        );

        let target_fp = CodeFingerprint {
            file_path: target.path.clone(),
            winnowing_hashes: winnowing::generate_fingerprints(
                &target.content,
                target.language,
                self.config.k_gram_size,
                self.config.window_size,
            ),
            fingerprint_lines,
            ast_hashes: ast::generate_ast_hashes(&target.content, target.language)
                .unwrap_or_default(),
            token_count: target.content.lines().count(),
            language: target.language,
        };

        let mut results = Vec::new();

        for (path, fp) in &self.fingerprints {
            if fp.language != target_fp.language {
                continue;
            }

            let winnowing_score = winnowing::jaccard_similarity(
                &target_fp.winnowing_hashes,
                &fp.winnowing_hashes,
            );

            let ast_score = ast::ast_jaccard_similarity(
                &target_fp.ast_hashes,
                &fp.ast_hashes,
            );

            let similarity_score = if target_fp.ast_hashes.is_empty() {
                winnowing_score
            } else {
                0.4 * winnowing_score + 0.6 * ast_score
            };

            if similarity_score >= self.config.threshold {
                let matched_chunks = find_matching_chunks(&target_fp, fp);
                results.push(SimilarityResult {
                    file_a: target.path.clone(),
                    file_b: path.clone(),
                    similarity_score,
                    winnowing_score,
                    ast_score,
                    matched_chunks,
                });
            }
        }

        results.sort_by(|a, b| {
            b.similarity_score
                .partial_cmp(&a.similarity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Get the number of indexed files
    pub fn indexed_count(&self) -> usize {
        self.fingerprints.len()
    }
}

/// Find matching code chunks between two fingerprints by intersecting
/// their winnowing fingerprint sets, expanding each match point to a
/// surrounding window of lines, and grouping overlapping windows into chunks.
fn find_matching_chunks(
    fp_a: &CodeFingerprint,
    fp_b: &CodeFingerprint,
) -> Vec<ChunkMatch> {
    use crate::core::types::ChunkMatch;

    const EXPAND_LINES: usize = 3; // lines to expand around each match point

    // Build a map from hash → lines in file B
    let mut hash_to_lines_b: std::collections::HashMap<u32, Vec<usize>> =
        std::collections::HashMap::new();
    for &(hash, line) in &fp_b.fingerprint_lines {
        hash_to_lines_b.entry(hash).or_default().push(line);
    }

    // Collect all matching line ranges (expanded around each match)
    let mut ranges: Vec<(usize, usize, usize, usize)> = Vec::new(); // (a_start, a_end, b_start, b_end)

    for &(hash, line_a) in &fp_a.fingerprint_lines {
        if let Some(lines_b) = hash_to_lines_b.get(&hash) {
            for &line_b in lines_b {
                let a_start = line_a.saturating_sub(EXPAND_LINES);
                let b_start = line_b.saturating_sub(EXPAND_LINES);
                ranges.push((
                    a_start.max(1),
                    line_a + EXPAND_LINES,
                    b_start.max(1),
                    line_b + EXPAND_LINES,
                ));
            }
        }
    }

    if ranges.is_empty() {
        return Vec::new();
    }

    // Sort by line_a start
    ranges.sort_by_key(|(sa, _, _, _)| *sa);
    ranges.dedup();

    // Merge overlapping ranges into chunks
    let mut chunks: Vec<ChunkMatch> = Vec::new();
    let mut merged: Option<(usize, usize, usize, usize)> = None;

    for &(sa, ea, sb, eb) in &ranges {
        match merged {
            None => merged = Some((sa, ea, sb, eb)),
            Some((ms, me, mb_start, mb_end)) => {
                // Merge if ranges overlap in file A
                if sa <= me + 2 {
                    merged = Some((
                        ms.min(sa),
                        me.max(ea),
                        mb_start.min(sb),
                        mb_end.max(eb),
                    ));
                } else {
                    // Output current merged chunk
                    let score = if me > ms && mb_end > mb_start {
                        let size_a = me - ms;
                        let size_b = mb_end - mb_start;
                        1.0 - (size_a.abs_diff(size_b) as f64 / size_a.max(size_b) as f64)
                    } else {
                        1.0
                    };
                    chunks.push(ChunkMatch {
                        line_a: ms,
                        line_end_a: me,
                        line_b: mb_start,
                        line_end_b: mb_end,
                        score,
                    });
                    merged = Some((sa, ea, sb, eb));
                }
            }
        }
    }

    // Close last chunk
    if let Some((ms, me, mb_start, mb_end)) = merged {
        let score = if me > ms && mb_end > mb_start {
            let size_a = me - ms;
            let size_b = mb_end - mb_start;
            1.0 - (size_a.abs_diff(size_b) as f64 / size_a.max(size_b) as f64)
        } else {
            1.0
        };
        chunks.push(ChunkMatch {
            line_a: ms,
            line_end_a: me,
            line_b: mb_start,
            line_end_b: mb_end,
            score,
        });
    }

    // Sort by score descending, then by size
    chunks.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| (b.line_end_a - b.line_a).cmp(&(a.line_end_a - a.line_a)))
    });

    // Keep top 5 chunks
    chunks.truncate(5);

    chunks
}
