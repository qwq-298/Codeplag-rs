use std::collections::HashMap;
use crate::core::types::{
    AnalyzerConfig, CodeFingerprint, SimilarityResult, SourceFile,
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

            // Generate AST fingerprints
            let ast_hashes = ast::generate_ast_hashes(&file.content, file.language)
                .unwrap_or_default();

            let fingerprint = CodeFingerprint {
                file_path: file.path.clone(),
                winnowing_hashes,
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
                    results.push(SimilarityResult {
                        file_a: fp_a.file_path.clone(),
                        file_b: fp_b.file_path.clone(),
                        similarity_score,
                        winnowing_score,
                        ast_score,
                        matched_chunks: Vec::new(), // TODO: implement chunk matching
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

        let target_fp = CodeFingerprint {
            file_path: target.path.clone(),
            winnowing_hashes: winnowing::generate_fingerprints(
                &target.content,
                target.language,
                self.config.k_gram_size,
                self.config.window_size,
            ),
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
                results.push(SimilarityResult {
                    file_a: target.path.clone(),
                    file_b: path.clone(),
                    similarity_score,
                    winnowing_score,
                    ast_score,
                    matched_chunks: Vec::new(),
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
