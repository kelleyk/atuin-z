use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "atuin-z", about = "Frecency-based directory jumping from Atuin history")]
pub struct Cli {
    /// List all matches with scores
    #[arg(short, long)]
    pub list: bool,

    /// Rank by frequency only
    #[arg(short, long)]
    pub rank: bool,

    /// Rank by recency only
    #[arg(short, long)]
    pub time: bool,

    /// Restrict to subdirectories of $ATUIN_Z_PWD
    #[arg(short, long)]
    pub current: bool,

    /// Add a path to the exclusion list
    #[arg(short = 'x', long)]
    pub exclude: bool,

    /// Override database path
    #[arg(long)]
    pub db: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,

    /// Keywords to match against directory paths
    pub keywords: Vec<String>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Output shell function for eval
    Init {
        /// Shell type
        shell: Shell,
    },
}

#[derive(Clone, clap::ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}
