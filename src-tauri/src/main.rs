#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use pixery_lib::cli;

#[derive(Parser)]
#[command(name = "pixery")]
#[command(about = "Unified image generation tool with CLI and GUI interfaces")]
#[command(long_about = "Unified image generation tool with CLI and GUI interfaces.\n\n\
    Subcommands provide CLI access; no args launches the GUI.\n\n\
    Workflow: generate → list/search → show (metadata) → view (image)\n\
    Iteration: generate → view → refine prompt or use --ref → generate\n\n\
    Supports Gemini, fal.ai, OpenAI, and self-hosted providers. Images are archived to \
    ~/media/image-gen/ with SQLite metadata tracking.\n\n\
    Model selection: Use 'pixery models' to list available models and 'pixery models MODEL --guide' \
    for prompting instructions specific to each model.")]
struct Args {
    #[command(subcommand)]
    command: Option<cli::Commands>,
}

fn main() {
    // Load ~/.env for API keys
    if let Some(home) = dirs::home_dir() {
        let env_path = home.join(".env");
        if env_path.exists() {
            let _ = dotenvy::from_path(&env_path);
        }
    }

    let args = Args::parse();

    match args.command {
        Some(cmd) => {
            // CLI mode
            if let Err(e) = cli::run(cmd) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        #[cfg(feature = "gui")]
        None => {
            // GUI mode
            pixery_lib::run()
        }
        #[cfg(not(feature = "gui"))]
        None => {
            // Headless build: no GUI to fall back to
            eprintln!("Built without the GUI. Run `pixery --help` for subcommands.");
            std::process::exit(2);
        }
    }
}
