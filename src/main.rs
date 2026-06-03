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

            let mut engine = SimilarityEngine::new(config);
            engine.index_files(&files);
            let results = engine.compare_all();

            print_results(&results, output);
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

            println!(
                "Comparing {} against {} files in {}",
                target_files[0].path,
                against_files.len(),
                against
            );

            let mut engine = SimilarityEngine::new(config);
            engine.index_files(&against_files);
            let results = engine.compare_against(&target_files[0]);

            print_results(&results, output);
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

            print_results(&results, output);
        }
    }
}

fn print_results(results: &[codeplag::core::types::SimilarityResult], format: &str) {
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
                println!();
            }

            println!("Found {} similar file pairs.", results.len());
        }
    }
}
