use std::collections::HashMap;
use crate::core::types::{
    AnalyzerConfig, ChunkMatch, CodeFingerprint, FunctionMatch,
    SimilarityResult, SourceFile,
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

            // Generate ALL k-gram hashes with line info for accurate chunk matching
            let all_kgraph_lines = winnowing::generate_all_kgraph_lines(
                &file.content,
                file.language,
                self.config.k_gram_size,
            );

            // Generate AST fingerprints
            let ast_hashes = ast::generate_ast_hashes(&file.content, file.language)
                .unwrap_or_default();

            let fingerprint = CodeFingerprint {
                file_path: file.path.clone(),
                winnowing_hashes,
                fingerprint_lines,
                all_kgraph_lines,
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
            all_kgraph_lines: winnowing::generate_all_kgraph_lines(
                &target.content,
                target.language,
                self.config.k_gram_size,
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

    /// Compare functions across files and return function-level matches.
    /// Each function is treated as a separate unit for fingerprinting and comparison.
    pub fn compare_functions(&self, files: &[SourceFile]) -> Vec<FunctionMatch> {
        use crate::core::types::{FunctionMatch, FunctionSnippet};

        let mut all_functions: Vec<(String, FunctionSnippet)> = Vec::new();

        // Step 1: Extract all functions from all files
        for file in files {
            let funcs = crate::fingerprint::ast::extract_functions(&file.content, file.language);
            for func in funcs {
                all_functions.push((file.path.clone(), func));
            }
        }

        if all_functions.is_empty() {
            return Vec::new();
        }

        // Step 2: Generate fingerprints for each function
        let mut func_fps: Vec<(String, &FunctionSnippet, CodeFingerprint)> = Vec::new();
        for (file_path, func) in &all_functions {
            let fp = CodeFingerprint {
                file_path: file_path.clone(),
                winnowing_hashes: winnowing::generate_fingerprints(
                    &func.content, func.language,
                    self.config.k_gram_size, self.config.window_size,
                ),
                fingerprint_lines: winnowing::generate_fingerprints_with_lines(
                    &func.content, func.language,
                    self.config.k_gram_size, self.config.window_size,
                ),
                all_kgraph_lines: winnowing::generate_all_kgraph_lines(
                    &func.content, func.language,
                    self.config.k_gram_size,
                ),
                ast_hashes: ast::generate_ast_hashes(&func.content, func.language)
                    .unwrap_or_default(),
                token_count: func.content.lines().count(),
                language: func.language,
            };
            func_fps.push((file_path.clone(), func, fp));
        }

        // Step 3: Compare all function pairs
        let mut results: Vec<FunctionMatch> = Vec::new();
        for i in 0..func_fps.len() {
            for j in (i + 1)..func_fps.len() {
                let (file_a, func_a, fp_a) = &func_fps[i];
                let (file_b, func_b, fp_b) = &func_fps[j];

                // Skip same file or different languages
                if fp_a.language != fp_b.language {
                    continue;
                }

                let winnowing_score = winnowing::jaccard_similarity(
                    &fp_a.winnowing_hashes, &fp_b.winnowing_hashes,
                );
                let ast_score = ast::ast_jaccard_similarity(
                    &fp_a.ast_hashes, &fp_b.ast_hashes,
                );
                let similarity_score = if fp_a.ast_hashes.is_empty() {
                    winnowing_score
                } else {
                    0.4 * winnowing_score + 0.6 * ast_score
                };

                if similarity_score >= self.config.threshold {
                    results.push(FunctionMatch {
                        func_a: func_a.name.clone(),
                        file_a: file_a.clone(),
                        lines_a: (func_a.start_line, func_a.end_line),
                        func_b: func_b.name.clone(),
                        file_b: file_b.clone(),
                        lines_b: (func_b.start_line, func_b.end_line),
                        similarity_score,
                        winnowing_score,
                        ast_score,
                    });
                }
            }
        }

        // Sort by similarity descending
        results.sort_by(|a, b| {
            b.similarity_score
                .partial_cmp(&a.similarity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }
}

/// Find matching code chunks between two fingerprints.
///
/// Uses a voting mechanism: each matching k-gram hash "votes" for an offset
/// (line_b - line_a). The offset with the most votes identifies the likely
/// aligned code block, even when functions are reordered.
fn find_matching_chunks(
    fp_a: &CodeFingerprint,
    fp_b: &CodeFingerprint,
) -> Vec<ChunkMatch> {
    use crate::core::types::ChunkMatch;
    use std::collections::HashMap;

    // Build hash → lines map for file B
    let mut hash_to_lines_b: HashMap<u32, Vec<usize>> = HashMap::new();
    for &(hash, line) in &fp_b.all_kgraph_lines {
        hash_to_lines_b.entry(hash).or_default().push(line);
    }

    // Voting: each matching hash votes for ALL (line_b - line_a) offsets
    let mut offset_votes: HashMap<i64, usize> = HashMap::new();
    let mut offset_pairs: HashMap<i64, Vec<(usize, usize)>> = HashMap::new();

    for &(hash, line_a) in &fp_a.all_kgraph_lines {
        if let Some(lines_b) = hash_to_lines_b.get(&hash) {
            for &line_b in lines_b {
                let offset = line_b as i64 - line_a as i64;
                *offset_votes.entry(offset).or_default() += 1;
                offset_pairs.entry(offset).or_default().push((line_a, line_b));
            }
        }
    }

    if offset_votes.is_empty() {
        return Vec::new();
    }

    // Pick the best offsets (most votes). Keep top 5.
    let mut ranked_offsets: Vec<(i64, usize)> = offset_votes.into_iter().collect();
    ranked_offsets.sort_by_key(|(_, v)| std::cmp::Reverse(*v));

    // Debug: print vote counts (can be removed later)
    ranked_offsets.truncate(5);

    let min_votes: usize = 2;
    let expand: usize = 3; // context lines after the matched region

    let mut chunks: Vec<ChunkMatch> = Vec::new();

    for &(offset, votes) in &ranked_offsets {
        if votes < min_votes {
            continue;
        }

        let pairs = match offset_pairs.get(&offset) {
            Some(p) => p,
            None => continue,
        };

        let mut sorted = pairs.clone();
        sorted.sort_by_key(|(a, _)| *a);
        sorted.dedup();

        // Group consecutive line_a into sub-chunks
        let mut sub_start: Option<(usize, usize, usize, usize)> = None;
        for &(la, lb) in &sorted {
            match sub_start {
                None => sub_start = Some((la, la, lb, lb)),
                Some((sa, ea, sb, eb)) => {
                    if la <= ea + 2 && (lb as i64 - sb as i64).abs() as usize <= eb - sb + 3 {
                        sub_start = Some((sa, la, sb.min(lb), eb.max(lb)));
                    } else {
                        chunks.push(ChunkMatch {
                            line_a: sa,
                            line_end_a: ea + expand,
                            line_b: sb,
                            line_end_b: eb + expand,
                            score: 0.0, // will be recomputed below
                        });
                        sub_start = Some((la, la, lb, lb));
                    }
                }
            }
        }
        if let Some((sa, ea, sb, eb)) = sub_start {
            chunks.push(ChunkMatch {
                line_a: sa,
                line_end_a: ea + expand,
                line_b: sb,
                line_end_b: eb + expand,
                score: 0.0, // will be recomputed below
            });
        }
    }

    // Compute actual similarity score for each chunk
    for chunk in &mut chunks {
        let hashes_a: std::collections::HashSet<u32> = fp_a.all_kgraph_lines
            .iter()
            .filter(|(_, l)| *l >= chunk.line_a && *l <= chunk.line_end_a)
            .map(|(h, _)| *h)
            .collect();
        let hashes_b: std::collections::HashSet<u32> = fp_b.all_kgraph_lines
            .iter()
            .filter(|(_, l)| *l >= chunk.line_b && *l <= chunk.line_end_b)
            .map(|(h, _)| *h)
            .collect();

        let intersection = hashes_a.intersection(&hashes_b).count();
        let union = hashes_a.len() + hashes_b.len() - intersection;
        chunk.score = if union > 0 {
            intersection as f64 / union as f64
        } else {
            0.0
        };
    }

    // Sort by score descending, then by size
    chunks.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| (b.line_end_a - b.line_a).cmp(&(a.line_end_a - a.line_a)))
    });

    // Deduplicate by line range
    chunks.dedup_by(|a, b| {
        a.line_a == b.line_a && a.line_end_a == b.line_end_a
            && a.line_b == b.line_b && a.line_end_b == b.line_end_b
    });
    chunks.truncate(5);

    chunks
}
