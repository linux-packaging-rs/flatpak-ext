use clap::{Parser, Subcommand};
use flatpak_ext::types::FlatpakExtError;
use run_temp::run_no_install;

pub mod run_temp;
pub mod utils;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
/// Flatpak-ext: Tools to extend flatpak's functionality
struct Cli {
    /// Command to run
    #[command(subcommand)]
    command: Command,
    /// Verbose
    #[arg(long)]
    verbose: bool,
}

#[derive(Subcommand, Clone, Debug)]
enum Command {
    /// Run a flatpak temporarily, without installing
    RunTemp {
        /// Flatpak to run from file
        #[arg(short, long)]
        file: Option<String>,
        /// Dependency file (leave out to download dependencies automatically)
        #[arg(short, long)]
        dep: Option<String>,
        /// Flatpak appid to download
        #[arg(short, long)]
        app_id: Option<String>,
        /// Flatpak remote to use to download any flatpaks (defaults to flathub)
        #[arg(short, long)]
        remote: Option<String>,
        /// Clean out the temp repo directory
        #[arg(short, long)]
        clean: bool,
    },
}

fn main() -> Result<(), FlatpakExtError> {
    let cli = Cli::parse();
    if cli.verbose {
        simple_logger::init_with_level(log::Level::Trace).unwrap();
    }
    log::info!("Starting flatrun!");
    match cli.command {
        Command::RunTemp {
            file,
            dep,
            app_id,
            remote,
            clean,
        } => run_no_install(file, dep, app_id, remote, clean)?,
    }
    Ok(())
}
