//! Leptos Init CLI Binary
//! 
//! Command-line interface for creating new Leptos projects with smart scaffolding.

use clap::Parser;
use leptos_init::{InitConfig, cli::LeptosInitCli, ProjectTemplate, ServerBackend, Database, Styling};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    name = "leptos-init",
    about = "Create a new Leptos project with smart scaffolding",
    version = "0.1.0"
)]
struct Cli {
    /// Project name
    name: String,

    /// Project template
    #[arg(long, value_enum, default_value = "fullstack")]
    template: ProjectTemplate,

    /// Server backend (for fullstack/api templates)
    #[arg(long, value_enum, default_value = "axum")]
    server: Option<ServerBackend>,

    /// Database integration
    #[arg(long, value_enum, default_value = "none")]
    database: Option<Database>,

    /// Styling framework
    #[arg(long, value_enum, default_value = "none")]
    styling: Option<Styling>,

    /// Enable tracing
    #[arg(long)]
    tracing: bool,

    /// Enable islands architecture
    #[arg(long)]
    islands: bool,

    /// Force overwrite existing directory
    #[arg(short, long)]
    force: bool,

    /// Target directory (defaults to current directory)
    #[arg(long)]
    target: Option<PathBuf>,

    /// Run in interactive mode
    #[arg(short, long)]
    interactive: bool,
}


fn main() {
    let cli = Cli::parse();

    // Validate project name
    if !is_valid_project_name(&cli.name) {
        eprintln!("❌ Error: Invalid project name '{}'", cli.name);
        eprintln!("   Project names must:");
        eprintln!("   - Only contain lowercase letters, numbers, underscores, and hyphens");
        eprintln!("   - Start with a letter");
        eprintln!("   - Be valid Rust identifiers");
        process::exit(1);
    }

    // Determine target path
    let target_path = cli.target
        .unwrap_or_else(|| std::env::current_dir().unwrap())
        .join(&cli.name);

    // Check if directory exists
    if target_path.exists() && !cli.force {
        eprintln!("❌ Error: Directory '{}' already exists", target_path.display());
        eprintln!("   Use --force to overwrite existing directory");
        process::exit(1);
    }

    // Remove existing directory if force is enabled
    if target_path.exists() && cli.force {
        std::fs::remove_dir_all(&target_path).unwrap_or_else(|e| {
            eprintln!("❌ Error: Failed to remove existing directory: {}", e);
            process::exit(1);
        });
    }

    // Create configuration
    let mut config = InitConfig::for_template(cli.name.clone(), cli.template.clone());

    // Apply CLI options
    if let Some(server) = cli.server {
        config.server = server;
    }
    if let Some(database) = cli.database {
        config.database = database;
    }
    if let Some(styling) = cli.styling {
        config.styling = styling;
    }
    if cli.tracing {
        config.use_tracing = true;
    }
    if cli.islands {
        config.use_islands = true;
    }

    // Run in interactive mode if requested
    if cli.interactive {
        if let Err(e) = LeptosInitCli::run_interactive(cli.name, &target_path) {
            eprintln!("❌ Error: {}", e);
            process::exit(1);
        }
        return;
    }

    // Generate project
    match generate_project(config, &target_path) {
        Ok(_) => {
            println!("✅ Project '{}' created successfully!", cli.name);
            display_next_steps(&cli.name);
        }
        Err(e) => {
            eprintln!("❌ Error creating project: {}", e);
            process::exit(1);
        }
    }
}

fn is_valid_project_name(name: &str) -> bool {
    // Must be non-empty
    if name.is_empty() {
        return false;
    }

    // Must start with a letter
    if !name.chars().next().unwrap().is_ascii_alphabetic() {
        return false;
    }

    // Must only contain lowercase letters, numbers, underscores, and hyphens
    name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

fn generate_project(config: InitConfig, target_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Creating Leptos {} project: {}", 
        match config.template {
            ProjectTemplate::Spa => "SPA",
            ProjectTemplate::Fullstack => "fullstack", 
            ProjectTemplate::Static => "static",
            ProjectTemplate::Api => "API",
            ProjectTemplate::Custom => "custom",
        },
        config.name
    );

    let generator = leptos_init::ProjectGenerator::new(config, target_path);
    generator.generate()?;

    Ok(())
}

fn display_next_steps(project_name: &str) {
    println!("\n📖 Next Steps:");
    println!("  1. cd {}", project_name);
    println!("  2. Run `cargo leptos watch` to start development");
    println!("  3. Open http://127.0.0.1:3000 in your browser");
    println!("\n📚 Resources:");
    println!("  • Documentation: https://leptos.dev");
    println!("  • Book: https://leptos-rs.github.io/leptos/");
    println!("  • Discord: https://discord.gg/YdRAhS7eQB");
    println!("\n🎉 Happy coding with Leptos!");
}
