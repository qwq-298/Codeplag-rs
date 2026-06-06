use std::path::PathBuf;
use clap::Parser;
use codeplag::cli::{Cli, Commands};
use codeplag::core::types::AnalyzerConfig;
use codeplag::engine::SimilarityEngine;
use codeplag::fetcher::github::GitHubFetcher;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    let cli = Cli::parse();

    if cli.verbose {
        log::set_max_level(log::LevelFilter::Debug);
    }

    let config = AnalyzerConfig {
        k_gram_size: cli.k_gram,
        window_size: cli.window,
        threshold: cli.threshold,
        ..Default::default()
    };

    let work_dir = PathBuf::from(".codeplag_work");

    match &cli.command {
        Commands::Analyze { path, output } => {
            let fetcher = GitHubFetcher::new(&work_dir);
            let files = fetcher.collect_local(path)
                .unwrap_or_else(|e| {
                    eprintln!("Error collecting files: {}", e);
                    std::process::exit(1);
                });

            println!("Found {} source files in {}", files.len(), path);

            if cli.functions {
                let engine = SimilarityEngine::new(config);
                let func_results = engine.compare_functions(&files);
                print_function_results(&func_results, output);
            } else {
                let mut engine = SimilarityEngine::new(config);
                engine.index_files(&files);
                let results = engine.compare_all();

                let file_refs: Vec<&codeplag::core::types::SourceFile> = files.iter().collect();
                print_results_with_files(&results, output, &file_refs);
            }
        }

        Commands::Compare { file, against, output } => {
            let fetcher = GitHubFetcher::new(&work_dir);
            let target_files = fetcher.collect_local(file)
                .unwrap_or_else(|e| {
                    eprintln!("Error reading target file: {}", e);
                    std::process::exit(1);
                });

            let against_files = fetcher.collect_local(against)
                .unwrap_or_else(|e| {
                    eprintln!("Error collecting comparison files: {}", e);
                    std::process::exit(1);
                });

            if target_files.is_empty() {
                eprintln!("No valid source file found at: {}", file);
                std::process::exit(1);
            }

            if against_files.len() == 1 {
                println!(
                    "Comparing {} <-> {}",
                    target_files[0].path,
                    against_files[0].path
                );
            } else {
                println!(
                    "Comparing {} against {} files in {}",
                    target_files[0].path,
                    against_files.len(),
                    against
                );
            }

            if cli.functions {
                // Function-level: merge target + against and compare all functions
                let all_files: Vec<codeplag::core::types::SourceFile> =
                    target_files.iter().chain(against_files.iter()).cloned().collect();
                let engine = SimilarityEngine::new(config);
                let func_results = engine.compare_functions(&all_files);
                // Filter to only target vs against (not against vs against)
                let filtered: Vec<_> = func_results.into_iter()
                    .filter(|r| {
                        r.file_a == target_files[0].path || r.file_b == target_files[0].path 
                    })
                    .collect();
                print_function_results(&filtered, output);
            } else {
                let mut engine = SimilarityEngine::new(config);
                engine.index_files(&against_files);
                let results = engine.compare_against(&target_files[0]);

                let all_files: Vec<&codeplag::core::types::SourceFile> =
                    target_files.iter().chain(against_files.iter()).collect();
                print_results_with_files(&results, output, &all_files);
            }
        }

        Commands::Fetch { repo, output } => {
            let fetcher = GitHubFetcher::new(&work_dir);
            let files = fetcher.fetch_repo(repo)
                .unwrap_or_else(|e| {
                    eprintln!("Error fetching repository: {}", e);
                    std::process::exit(1);
                });

            println!("Fetched {} source files from {}", files.len(), repo);

            let mut engine = SimilarityEngine::new(config);
            engine.index_files(&files);
            let results = engine.compare_all();

            let file_refs: Vec<&codeplag::core::types::SourceFile> = files.iter().collect();
            print_results_with_files(&results, output, &file_refs);
        }

        Commands::Project { project_a, project_b, output } => {
            let fetcher = GitHubFetcher::new(&work_dir);
            let files_a = fetcher.collect_local(project_a)
                .unwrap_or_else(|e| {
                    eprintln!("Error reading project A: {}", e);
                    std::process::exit(1);
                });
            let files_b = fetcher.collect_local(project_b)
                .unwrap_or_else(|e| {
                    eprintln!("Error reading project B: {}", e);
                    std::process::exit(1);
                });

            println!(
                "Comparing project A ({} files) vs project B ({} files)",
                files_a.len(),
                files_b.len()
            );

            let engine = SimilarityEngine::new(config);
            let result = engine.compare_projects(&files_a, &files_b);
            print_project_result(&result, output);
        }
    }
}

fn print_results_with_files(
    results: &[codeplag::core::types::SimilarityResult],
    format: &str,
    all_files: &[&codeplag::core::types::SourceFile],
) {
    // Build content lookup by path
    let content_map: std::collections::HashMap<&str, &str> = all_files
        .iter()
        .map(|f| (f.path.as_str(), f.content.as_str()))
        .collect();

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(results)
                .unwrap_or_else(|e| format!("Error serializing: {}", e));
            println!("{}", json);
        }
        _ => {
            if results.is_empty() {
                println!("\nNo similar file pairs found.");
                return;
            }

            println!("\n=== Similarity Results ===\n");
            for (i, result) in results.iter().enumerate() {
                println!(
                    "{}. {} <-> {}",
                    i + 1,
                    result.file_a,
                    result.file_b
                );
                println!(
                    "   Overall:  {:.1}%",
                    result.similarity_score * 100.0
                );
                println!(
                    "   Winnowing: {:.1}% | AST: {:.1}%",
                    result.winnowing_score * 100.0,
                    result.ast_score * 100.0
                );

                // Display matched chunks
                if !result.matched_chunks.is_empty() {
                    let lines_a: Vec<&str> = content_map
                        .get(result.file_a.as_str())
                        .map(|c| c.lines().collect())
                        .unwrap_or_default();
                    let lines_b: Vec<&str> = content_map
                        .get(result.file_b.as_str())
                        .map(|c| c.lines().collect())
                        .unwrap_or_default();
                    print_chunks(&result.matched_chunks, &lines_a, &lines_b);
                }
                println!();
            }

            println!("Found {} similar file pairs.", results.len());
        }
    }
}

/// Display matched chunks in a visual side-by-side format
fn print_chunks(
    chunks: &[codeplag::core::types::ChunkMatch],
    lines_a: &[&str],
    lines_b: &[&str],
) {
    const MAX_WIDTH: usize = 50;

    for (chunk_idx, chunk) in chunks.iter().enumerate() {
        println!();
        println!(
            "   ── Chunk {} ({:.0}% match, lines {}-{} ↔ {}-{}) ──",
            chunk_idx + 1,
            chunk.score * 100.0,
            chunk.line_a,
            chunk.line_end_a,
            chunk.line_b,
            chunk.line_end_b,
        );

        let max_lines = (chunk.line_end_a - chunk.line_a + 1)
            .max(chunk.line_end_b - chunk.line_b + 1);

        for offset in 0..max_lines {
            let idx_a = chunk.line_a.saturating_sub(1).saturating_add(offset);
            let idx_b = chunk.line_b.saturating_sub(1).saturating_add(offset);

            let left = format!(
                "{:>4} | {}",
                idx_a + 1,
                truncate(lines_a.get(idx_a).copied().unwrap_or(""), MAX_WIDTH)
            );
            let right = format!(
                "{:>4} | {}",
                idx_b + 1,
                truncate(lines_b.get(idx_b).copied().unwrap_or(""), MAX_WIDTH)
            );

            println!("   {:<54}  {}", left, right);
        }
    }
}

/// Display function-level comparison results
fn print_function_results(
    results: &[codeplag::core::types::FunctionMatch],
    format: &str,
) {
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(results)
                .unwrap_or_else(|e| format!("Error serializing: {}", e));
            println!("{}", json);
        }
        _ => {
            if results.is_empty() {
                println!("\nNo similar functions found.");
                return;
            }

            println!("\n=== Function-Level Similarity Results ===\n");
            for (i, r) in results.iter().enumerate() {
                println!(
                    "{}: {}() [{}:{}-{}] ↔ {}() [{}:{}-{}]",
                    i + 1,
                    r.func_a, r.file_a, r.lines_a.0, r.lines_a.1,
                    r.func_b, r.file_b, r.lines_b.0, r.lines_b.1,
                );
                println!(
                    "   Overall:  {:.1}%  |  Winnowing: {:.1}%  |  AST: {:.1}%",
                    r.similarity_score * 100.0,
                    r.winnowing_score * 100.0,
                    r.ast_score * 100.0,
                );
                println!();
            }

            println!("Found {} similar function pairs.", results.len());
        }
    }
}

/// Display project-level comparison results
fn print_project_result(
    result: &codeplag::core::types::ProjectResult,
    format: &str,
) {
    match format {
        "json" => {
            let json = serde_json::to_string_pretty(result)
                .unwrap_or_else(|e| format!("Error serializing: {}", e));
            println!("{}", json);
        }
        _ => {
            if result.file_matches.is_empty() {
                println!("\nNo similar files found between the two projects.");
                return;
            }

            println!("\n=== Project Comparison Results ===\n");

            for (i, m) in result.file_matches.iter().enumerate() {
                println!(
                    "{:>3}. {: <40} ↔ {}",
                    i + 1,
                    m.file_a,
                    m.file_b,
                );
                println!(
                    "      {:.1}%  |  Winnowing: {:.1}%  |  AST: {:.1}%",
                    m.similarity_score * 100.0,
                    m.winnowing_score * 100.0,
                    m.ast_score * 100.0,
                );
                println!();
            }

            println!(
                "─────────────────────────────────────────────"
            );
            println!(
                "  Project Similarity: {:.1}%  (avg of {} file matches)",
                result.project_score * 100.0,
                result.file_matches.len(),
            );
        }
    }
}

/// Truncate a string to max_width chars, adding "…" if cut.
/// Uses char boundaries to avoid panics with multi-byte UTF-8.
fn truncate(s: &str, max_width: usize) -> String {
    if s.chars().count() <= max_width {
        format!("{: <max_width$}", s, max_width = max_width)
    } else {
        // Find byte offset of the (max_width - 1)-th char
        let byte_end = s
            .char_indices()
            .nth(max_width - 1)
            .map(|(idx, _)| idx)
            .unwrap_or(s.len());
        format!("{}…", &s[..byte_end])
    }
}
