use clap::{Parser, Subcommand};

/// Codeplag - A GitHub code plagiarism analyzer
#[derive(Parser, Debug)]
#[command(name = "codeplag")]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Minimum similarity threshold [0.0, 1.0]
    #[arg(short, long, default_value = "0.5")]
    pub threshold: f64,

    /// k-gram size for winnowing
    #[arg(long, default_value = "5")]
    pub k_gram: usize,

    /// Window size for winnowing
    #[arg(long, default_value = "4")]
    pub window: usize,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Analyze a local directory for code similarity
    Analyze {
        /// Path to the directory to analyze
        #[arg(short, long)]
        path: String,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },

    /// Compare a single file against a directory
    Compare {
        /// Path to the target file
        #[arg(short, long)]
        file: String,

        /// Path to the directory to compare against
        #[arg(short, long)]
        against: String,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },

    /// Fetch and analyze a GitHub repository
    Fetch {
        /// GitHub repository URL
        #[arg(short, long)]
        repo: String,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        output: String,
    },
}
