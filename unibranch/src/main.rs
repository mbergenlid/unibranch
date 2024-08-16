use anyhow::Context;
use clap::{command, Parser, Subcommand};
use tracing_subscriber::EnvFilter;
use ubr::{
    commands::{create, pull, push},
    git::{CommandOption, GitRepo},
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long)]
    quiet: bool,

    #[arg(short, long)]
    dry_run: bool,
}

#[derive(Subcommand)]
enum Commands {
    Create(create::Options),
    Pull(pull::Options),
    Push,
}

fn main() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(false)
        .with_line_number(false)
        .with_thread_ids(false)
        .with_target(false)
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    let cli = Cli::parse();

    let remote_option = if cli.dry_run {
        CommandOption::DryRun
    } else if cli.quiet {
        CommandOption::Silent
    } else {
        CommandOption::Default
    };
    let git_repo = GitRepo::open_with_remote(".", remote_option).context("Opening GIT repo")?;

    match cli.command {
        Commands::Create(config) => create::execute(config, git_repo)?,
        Commands::Pull(config) => pull::execute(config, git_repo)?,
        Commands::Push => push::execute(".")?,
    };
    Ok(())
}
