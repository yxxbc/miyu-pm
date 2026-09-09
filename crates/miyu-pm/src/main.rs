mod actions;
mod config;
mod paths;
mod registry;
mod source;
mod state;
mod types;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "miyu-pm",
    version,
    about = "miyu third-party plugin / MCP package manager (M1 prototype)"
)]
struct Cli {
    /// Override miyu home directory (default: ~/.miyu)
    #[arg(long, global = true)]
    miyu_home: Option<PathBuf>,

    /// Override miyu-pm home directory (default: ~/.miyu-pm)
    #[arg(long, global = true)]
    pm_home: Option<PathBuf>,

    /// Path to the registry index.json (default: ./registry/index.json)
    #[arg(long, global = true)]
    registry: Option<PathBuf>,

    /// Show what would happen without writing anything
    #[arg(long, global = true)]
    dry_run: bool,

    /// Assume yes for every confirmation
    #[arg(long, global = true)]
    yes: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Search packages in the registry
    Search { query: String },
    /// Show details about one package
    Info { name: String },
    /// List installed packages (or all with --all)
    List {
        /// Show all packages in the registry too
        #[arg(long)]
        all: bool,
    },
    /// Install an mcp/skill/script package from the registry
    Install {
        name: String,
        /// Override the git repository URL
        #[arg(long)]
        repo: Option<String>,
        /// Skip the package setup commands after clone
        #[arg(long)]
        no_setup: bool,
        /// Supply env value to resolve placeholders, e.g. KEY=VALUE
        #[arg(long = "env", value_name = "KEY=VAL")]
        envs: Vec<String>,
    },
    /// Remove an installed package
    Remove { name: String },
    /// Reload the registry index into the local cache
    Update,
    /// Upgrade one or more installed packages (all when no names given)
    Upgrade {
        #[arg(value_name = "NAME")]
        names: Vec<String>,
        /// Skip setup commands after pulling
        #[arg(long)]
        no_setup: bool,
    },
    /// Run basic static audit of a registry package
    Audit { name: String },
    /// Check local miyu state consistency
    Doctor,
    /// Show environment summary
    Status,
    /// Update miyu-pm itself from a GitHub release
    SelfUpdate,
    /// Manage stored registry sources
    Source {
        #[command(subcommand)]
        action: SourceAction,
    },
}

#[derive(Debug, Subcommand)]
enum SourceAction {
    /// List the active registry and stored sources
    List,
    /// Add or update a stored source (http(s) URL or local index.json path)
    Add {
        /// Optional short name (defaults to the last URL segment)
        #[arg(long)]
        name: Option<String>,
        /// Source URL or local path to index.json
        url: String,
    },
    /// Remove a stored source by name or URL
    Remove { name_or_url: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let paths = paths::Paths::resolve(cli.miyu_home, cli.pm_home, cli.registry)?;
    let opts = actions::Options {
        dry_run: cli.dry_run,
        yes: cli.yes,
    };

    match cli.command {
        Command::Search { query } => actions::search(&paths, &query),
        Command::Info { name } => actions::info(&paths, &name),
        Command::List { all } => actions::list(&paths, all),
        Command::Install {
            name,
            repo,
            no_setup,
            envs,
        } => actions::install(&paths, &name, repo.as_deref(), no_setup, &envs, opts),
        Command::Remove { name } => actions::remove(&paths, &name, opts),
        Command::Update => actions::update(&paths),
        Command::Upgrade { names, no_setup } => actions::upgrade(&paths, &names, no_setup, opts),
        Command::Audit { name } => actions::audit(&paths, &name),
        Command::Doctor => actions::doctor(&paths),
        Command::Status => actions::status(&paths),
        Command::SelfUpdate => actions::self_update(&paths, opts),
        Command::Source { action } => match action {
            SourceAction::List => source::list(&paths),
            SourceAction::Add { name, url } => source::add(&paths, name, url, opts),
            SourceAction::Remove { name_or_url } => source::remove(&paths, &name_or_url),
        },
    }
}
