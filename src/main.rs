mod cli;
mod db;
mod exclusions;
mod frecency;
mod matching;
mod shell;

use anyhow::{bail, Result};
use clap::Parser;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ns() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_nanos() as i64
}

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    // Handle `init` subcommand
    if let Some(cli::Command::Init { shell: s }) = &cli.command {
        print!("{}", shell::init(s));
        return Ok(());
    }

    // Handle `-x` / `--exclude`
    if cli.exclude {
        if cli.keywords.is_empty() {
            bail!("atuin-z -x requires a path argument");
        }
        for path in &cli.keywords {
            exclusions::add(path)?;
        }
        return Ok(());
    }

    // Resolve and open DB
    let db_path = db::resolve_db_path(cli.db.as_deref())?;
    let conn = db::open(&db_path)?;

    // Determine cwd prefix for `-c` flag
    let cwd_prefix = if cli.current {
        std::env::var("ATUIN_Z_PWD").ok()
    } else {
        None
    };

    // Query
    let entries = db::query_dirs(&conn, cwd_prefix.as_deref())?;

    // Determine scoring mode
    let mode = if cli.rank {
        frecency::Mode::Frequency
    } else if cli.time {
        frecency::Mode::Recency
    } else {
        frecency::Mode::Frecency
    };

    // Load exclusions
    let exclusion_list = exclusions::load()?;

    // Rank
    let now = now_ns();
    let results = matching::rank(entries, &cli.keywords, &mode, now, &exclusion_list);

    if cli.list {
        for r in &results {
            println!("{:>10.1}  {}", r.score, r.path);
        }
    } else if let Some(best) = results.first() {
        println!("{}", best.path);
    }

    Ok(())
}
