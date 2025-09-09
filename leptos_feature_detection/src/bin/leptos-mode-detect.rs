//! CLI tool for detecting and configuring Leptos build modes
//!
//! This tool analyzes Leptos projects and provides intelligent mode detection,
//! configuration validation, and migration assistance.

use clap::Parser;
use leptos_feature_detection::{
    ModeDetector, LeptosMode, 
    detection::SmartDetector, 
    Severity
};
use leptos_feature_detection::validation::ValidationFramework;
use std::path::PathBuf;
use std::process;

#[derive(clap::Parser)]
#[command(name = "leptos-mode-detect")]
#[command(about = "Intelligent Leptos build mode detection and configuration")]
#[command(version)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Analyze project and detect appropriate mode
    Detect {
        /// Project directory (defaults to current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,
        
        /// Output format
        #[arg(short, long, default_value = "human")]
        format: OutputFormat,
        
        /// Show detailed analysis
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Validate current configuration
    Validate {
        /// Project directory (defaults to current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,
        
        /// Mode to validate against (auto-detect if not specified)
        #[arg(short, long)]
        mode: Option<String>,
        
        /// Fail on warnings
        #[arg(long)]
        strict: bool,
    },
    
    /// Migrate from manual feature flags to mode system
    Migrate {
        /// Project directory (defaults to current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,
        
        /// Target mode (auto-detect if not specified)
        #[arg(short, long)]
        mode: Option<String>,
        
        /// Dry run - show changes without applying
        #[arg(long)]
        dry_run: bool,
        
        /// Skip backup creation
        #[arg(long)]
        no_backup: bool,
    },
    
    /// Show recommended configuration for detected mode
    Configure {
        /// Project directory (defaults to current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,
        
        /// Target mode (auto-detect if not specified)
        #[arg(short, long)]
        mode: Option<String>,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum OutputFormat {
    Human,
    Json,
    Yaml,
}

fn main() {
    let args = Args::parse();
    
    let result = match args.command {
        Commands::Detect { path, format, verbose } => {
            detect_mode(path.unwrap_or_else(|| PathBuf::from(".")), format, verbose)
        }
        Commands::Validate { path, mode, strict } => {
            validate_configuration(
                path.unwrap_or_else(|| PathBuf::from(".")),
                mode,
                strict,
            )
        }
        Commands::Migrate { path, mode, dry_run, no_backup } => {
            migrate_project(
                path.unwrap_or_else(|| PathBuf::from(".")),
                mode,
                dry_run,
                no_backup,
            )
        }
        Commands::Configure { path, mode } => {
            show_configuration(
                path.unwrap_or_else(|| PathBuf::from(".")),
                mode,
            )
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn detect_mode(project_path: PathBuf, format: OutputFormat, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Analyzing Leptos project: {}", project_path.display());
    
    let detector = SmartDetector::new(&project_path);
    let analysis = detector.analyze_comprehensive()?;

    match format {
        OutputFormat::Human => {
            println!("\n📊 Detection Results:");
            println!("   Mode: {} (confidence: {:.1}%)", 
                mode_display(&analysis.detected_mode), 
                analysis.confidence * 100.0
            );

            if !analysis.current_features.is_empty() {
                println!("   Current features: [{}]", analysis.current_features.join(", "));
            }

            if verbose {
                if !analysis.issues.is_empty() {
                    println!("\n⚠️  Issues Found:");
                    for issue in &analysis.issues {
                        let icon = match issue.severity {
                            Severity::Error => "❌",
                            Severity::Warning => "⚠️",
                            Severity::Info => "ℹ️",
                        };
                        println!("   {} {}", icon, issue.message);
                        if let Some(suggestion) = &issue.suggestion {
                            println!("      💡 {}", suggestion);
                        }
                    }
                }

                if !analysis.recommendations.is_empty() {
                    println!("\n💡 Recommendations:");
                    for rec in &analysis.recommendations {
                        println!("   • {}: {}", rec.action, rec.rationale);
                    }
                }
            }
        }
        OutputFormat::Json => {
            let output = serde_json::json!({
                "detected_mode": analysis.detected_mode,
                "confidence": analysis.confidence,
                "current_features": analysis.current_features,
                "issues": analysis.issues.iter().map(|i| serde_json::json!({
                    "severity": format!("{:?}", i.severity),
                    "message": i.message,
                    "suggestion": i.suggestion
                })).collect::<Vec<_>>(),
                "recommendations": analysis.recommendations.iter().map(|r| serde_json::json!({
                    "action": r.action,
                    "rationale": r.rationale,
                    "after": r.after
                })).collect::<Vec<_>>()
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        OutputFormat::Yaml => {
            let output = serde_yaml::to_string(&serde_json::json!({
                "detected_mode": analysis.detected_mode,
                "confidence": analysis.confidence,
                "current_features": analysis.current_features,
            }))?;
            print!("{}", output);
        }
    }

    Ok(())
}

fn validate_configuration(
    project_path: PathBuf, 
    mode_str: Option<String>, 
    strict: bool
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Validating Leptos project configuration...");
    
    let mode = if let Some(mode_str) = mode_str {
        parse_mode(&mode_str)?
    } else {
        // Auto-detect mode
        let detector = ModeDetector::new(&project_path);
        let analysis = detector.analyze()?;
        analysis.detected_mode
    };

    let validator = ValidationFramework::new(&project_path);
    let issues = validator.validate(&mode)?;

    if issues.is_empty() {
        println!("✅ Configuration is valid for {} mode!", mode_display(&mode));
        return Ok(());
    }

    println!("\n📋 Validation Results for {} mode:", mode_display(&mode));
    
    let mut error_count = 0;
    let mut warning_count = 0;
    
    for issue in &issues {
        let (icon, label) = match issue.severity {
            Severity::Error => { error_count += 1; ("❌", "ERROR") }
            Severity::Warning => { warning_count += 1; ("⚠️", "WARN") }
            Severity::Info => ("ℹ️", "INFO"),
        };
        
        println!("   {} [{}] {}", icon, label, issue.message);
        if let Some(file) = &issue.file {
            println!("       📁 {}", file.display());
        }
        if let Some(suggestion) = &issue.suggestion {
            println!("       💡 {}", suggestion);
        }
        println!();
    }

    println!("Summary: {} errors, {} warnings", error_count, warning_count);

    if error_count > 0 || (strict && warning_count > 0) {
        process::exit(1);
    }

    Ok(())
}

fn migrate_project(
    project_path: PathBuf,
    mode_str: Option<String>,
    dry_run: bool,
    no_backup: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Starting migration to Leptos mode system...");
    
    // Detect current mode if not specified
    let target_mode = if let Some(mode_str) = mode_str {
        parse_mode(&mode_str)?
    } else {
        let detector = SmartDetector::new(&project_path);
        let analysis = detector.analyze_comprehensive()?;
        println!("   🎯 Auto-detected target mode: {}", mode_display(&analysis.detected_mode));
        analysis.detected_mode
    };

    // Read current Cargo.toml
    let cargo_toml_path = project_path.join("Cargo.toml");
    let original_content = std::fs::read_to_string(&cargo_toml_path)?;
    let mut cargo_toml: toml::Value = toml::from_str(&original_content)?;

    // Create backup unless disabled
    if !no_backup && !dry_run {
        let backup_path = project_path.join("Cargo.toml.backup");
        std::fs::write(&backup_path, &original_content)?;
        println!("   💾 Created backup: {}", backup_path.display());
    }

    // Generate new configuration
    let new_config = generate_mode_config(&target_mode);
    
    // Apply changes to TOML
    apply_mode_migration(&mut cargo_toml, &target_mode, &new_config)?;
    
    let new_content = toml::to_string_pretty(&cargo_toml)?;

    if dry_run {
        println!("\n📋 Migration Preview (--dry-run):");
        println!("   Target mode: {}", mode_display(&target_mode));
        println!("\n   Changes to Cargo.toml:");
        println!("   {}", format_diff(&original_content, &new_content));
    } else {
        std::fs::write(&cargo_toml_path, &new_content)?;
        println!("✅ Migration completed!");
        println!("   🎯 Configured for: {}", mode_display(&target_mode));
        println!("   📝 Updated: {}", cargo_toml_path.display());
        
        // Validate the result
        println!("\n🔍 Validating migrated configuration...");
        let validator = ValidationFramework::new(&project_path);
        let issues = validator.validate(&target_mode)?;
        
        if issues.is_empty() {
            println!("✅ Migration successful - no validation issues!");
        } else {
            println!("⚠️ Migration completed with {} validation issues", issues.len());
            for issue in issues.iter().take(3) {
                println!("   • {}", issue.message);
            }
            if issues.len() > 3 {
                println!("   ... and {} more (run 'leptos-mode-detect validate' for details)", issues.len() - 3);
            }
        }
    }

    Ok(())
}

fn show_configuration(
    project_path: PathBuf,
    mode_str: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let target_mode = if let Some(mode_str) = mode_str {
        parse_mode(&mode_str)?
    } else {
        let detector = ModeDetector::new(&project_path);
        let analysis = detector.analyze()?;
        analysis.detected_mode
    };

    println!("📋 Recommended configuration for {} mode:", mode_display(&target_mode));
    println!();
    
    // Show mode declaration
    println!("   Add to Cargo.toml:");
    println!("   ```toml");
    println!("   [package.metadata.leptos]");
    println!("   mode = \"{}\"", mode_toml_value(&target_mode));
    
    // Show additional configuration based on mode
    match target_mode {
        LeptosMode::Fullstack => {
            println!("   site-root = \"target/site\"");
            println!("   site-pkg-dir = \"pkg\"");
            println!("   site-addr = \"127.0.0.1:3000\"");
        }
        LeptosMode::CSR => {
            println!("   output-name = \"{}\"", project_path.file_name().unwrap().to_string_lossy());
        }
        LeptosMode::Static => {
            println!("   site-root = \"dist\"");
            println!("   assets-dir = \"public\"");
        }
        _ => {}
    }
    println!("   ```");

    // Show required crate types
    let crate_types = target_mode.lib_features();
    if !crate_types.is_empty() {
        println!("\n   Required crate types:");
        println!("   ```toml");
        println!("   [lib]");
        match target_mode {
            LeptosMode::CSR | LeptosMode::Hydrate => {
                println!("   crate-type = [\"cdylib\"]");
            }
            LeptosMode::Fullstack => {
                println!("   crate-type = [\"cdylib\", \"rlib\"]");
            }
            _ => {
                println!("   crate-type = [\"rlib\"]");
            }
        }
        println!("   ```");
    }

    // Show build commands
    println!("\n   Build commands:");
    match target_mode {
        LeptosMode::CSR => {
            println!("   cargo leptos build --lib-only");
        }
        LeptosMode::SSR => {
            println!("   cargo leptos build --bin-only");
        }
        LeptosMode::Fullstack => {
            println!("   cargo leptos build");
            println!("   cargo leptos watch  # for development");
        }
        _ => {
            println!("   cargo leptos build");
        }
    }

    Ok(())
}

fn parse_mode(mode_str: &str) -> Result<LeptosMode, String> {
    match mode_str.to_lowercase().as_str() {
        "csr" | "client" => Ok(LeptosMode::CSR),
        "ssr" | "server" => Ok(LeptosMode::SSR),
        "hydrate" | "hydration" => Ok(LeptosMode::Hydrate),
        "fullstack" | "full-stack" | "isomorphic" => Ok(LeptosMode::Fullstack),
        "static" | "ssg" => Ok(LeptosMode::Static),
        "islands" => Ok(LeptosMode::Islands),
        _ => Err(format!("Unknown mode: {}. Valid modes: csr, ssr, hydrate, fullstack, static, islands", mode_str)),
    }
}

fn mode_display(mode: &LeptosMode) -> &str {
    match mode {
        LeptosMode::CSR => "Client-Side Rendering",
        LeptosMode::SSR => "Server-Side Rendering", 
        LeptosMode::Hydrate => "Hydration",
        LeptosMode::Fullstack => "Fullstack",
        LeptosMode::Static => "Static Site Generation",
        LeptosMode::Islands => "Islands",
    }
}

fn mode_toml_value(mode: &LeptosMode) -> &str {
    match mode {
        LeptosMode::CSR => "csr",
        LeptosMode::SSR => "ssr",
        LeptosMode::Hydrate => "hydrate", 
        LeptosMode::Fullstack => "fullstack",
        LeptosMode::Static => "static",
        LeptosMode::Islands => "islands",
    }
}

fn generate_mode_config(mode: &LeptosMode) -> toml::Value {
    let mut config = toml::map::Map::new();
    config.insert("mode".to_string(), toml::Value::String(mode_toml_value(mode).to_string()));
    
    match mode {
        LeptosMode::Fullstack => {
            config.insert("site-root".to_string(), toml::Value::String("target/site".to_string()));
            config.insert("site-pkg-dir".to_string(), toml::Value::String("pkg".to_string()));
            config.insert("site-addr".to_string(), toml::Value::String("127.0.0.1:3000".to_string()));
        }
        LeptosMode::Static => {
            config.insert("site-root".to_string(), toml::Value::String("dist".to_string()));
        }
        _ => {}
    }
    
    toml::Value::Table(config)
}

fn apply_mode_migration(
    cargo_toml: &mut toml::Value,
    target_mode: &LeptosMode,
    new_config: &toml::Value,
) -> Result<(), Box<dyn std::error::Error>> {
    // Remove old feature flags
    if let Some(features) = cargo_toml.get_mut("features") {
        if let Some(features_table) = features.as_table_mut() {
            if let Some(default_features) = features_table.get_mut("default") {
                if let Some(default_array) = default_features.as_array_mut() {
                    default_array.retain(|feature| {
                        if let Some(feature_str) = feature.as_str() {
                            !matches!(feature_str, "csr" | "ssr" | "hydrate" | "islands")
                        } else {
                            true
                        }
                    });
                }
            }
        }
    }

    // Add leptos metadata
    let package = cargo_toml.get_mut("package").ok_or("Missing [package] section")?;
    let package_table = package.as_table_mut().ok_or("Invalid [package] section")?;
    
    let metadata = package_table.entry("metadata".to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let metadata_table = metadata.as_table_mut().ok_or("Invalid metadata section")?;
    
    metadata_table.insert("leptos".to_string(), new_config.clone());

    // Update crate types if needed
    match target_mode {
        LeptosMode::CSR | LeptosMode::Hydrate => {
            ensure_crate_type(cargo_toml, vec!["cdylib"])?;
        }
        LeptosMode::Fullstack => {
            ensure_crate_type(cargo_toml, vec!["cdylib", "rlib"])?;
        }
        LeptosMode::SSR | LeptosMode::Static => {
            ensure_crate_type(cargo_toml, vec!["rlib"])?;
        }
        LeptosMode::Islands => {
            ensure_crate_type(cargo_toml, vec!["cdylib", "rlib"])?;
        }
    }

    Ok(())
}

fn ensure_crate_type(cargo_toml: &mut toml::Value, types: Vec<&str>) -> Result<(), Box<dyn std::error::Error>> {
      let lib = cargo_toml.get_mut("lib")
        .get_or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let lib_table = lib.as_table_mut().ok_or("Invalid [lib] section")?;
    
    let crate_types: Vec<toml::Value> = types.into_iter()
        .map(|t| toml::Value::String(t.to_string()))
        .collect();
    
    lib_table.insert("crate-type".to_string(), toml::Value::Array(crate_types));
    
    Ok(())
}

fn format_diff(old: &str, new: &str) -> String {
    // Simple diff formatting - in a real implementation, use a proper diff library
    format!("   Old: {} lines\n   New: {} lines", 
            old.lines().count(), 
            new.lines().count())
}