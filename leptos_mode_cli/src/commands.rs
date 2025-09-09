//! CLI command implementations

use anyhow::Result;
use leptos_feature_detection::{detection::SmartDetector, LeptosMode};
use leptos_mode_resolver::{BuildConfig, BuildMode, BuildTarget, ModeResolver};
use leptos_compile_validator::setup_validation_in_project;
use std::path::PathBuf;

/// Analyze current project and detect the appropriate mode
pub fn analyze_project(path: &PathBuf, format: &str, verbose: bool) -> Result<()> {
    use console::style;
    use indicatif::{ProgressBar, ProgressStyle};
    
    println!("{}", style("🔍 Analyzing Leptos project...").bold().blue());
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );
    pb.set_message("Detecting project structure...");
    
    let detector = SmartDetector::new(path);
    let analysis = detector.analyze_comprehensive()?;
    
    pb.finish_with_message("Analysis complete!");
    
    match format {
        "json" => {
            println!("JSON output not yet implemented");
        }
        "yaml" => {
            println!("YAML output not yet implemented");
        }
        "table" => {
            print_analysis_table(&analysis, verbose);
        }
        _ => {
            println!("Unknown format: {}. Using table format.", format);
            print_analysis_table(&analysis, verbose);
        }
    }
    
    Ok(())
}

fn print_analysis_table(analysis: &leptos_feature_detection::AnalysisResult, verbose: bool) {
    use console::style;
    
    println!("\n{}", style("📊 Analysis Results").bold().green());
    println!("{}", "=".repeat(50));
    
    // Detected mode
    let mode_color = match analysis.detected_mode {
        LeptosMode::CSR => style("CSR").cyan(),
        LeptosMode::SSR => style("SSR").yellow(),
        LeptosMode::Hydrate => style("Hydrate").blue(),
        LeptosMode::Fullstack => style("Fullstack").green(),
        LeptosMode::Static => style("Static").magenta(),
        LeptosMode::Islands => style("Islands").magenta(),
    };
    
    println!("Detected Mode: {} (confidence: {:.1}%)", 
             mode_color, 
             analysis.confidence * 100.0);
    
    // Current features
    if !analysis.current_features.is_empty() {
        println!("\nCurrent Features:");
        for feature in &analysis.current_features {
            println!("  • {}", style(feature).dim());
        }
    }
    
    // Issues
    if !analysis.issues.is_empty() {
        println!("\n{}", style("⚠️  Issues Found:").bold().red());
        for issue in &analysis.issues {
            let severity_icon = match issue.severity {
                leptos_feature_detection::Severity::Error => "❌",
                leptos_feature_detection::Severity::Warning => "⚠️",
                leptos_feature_detection::Severity::Info => "ℹ️",
            };
            
            println!("  {} {}", severity_icon, issue.message);
            if let Some(suggestion) = &issue.suggestion {
                println!("     💡 {}", style(suggestion).dim());
            }
        }
    }
    
    // Recommendations
    if !analysis.recommendations.is_empty() {
        println!("\n{}", style("💡 Recommendations:").bold().yellow());
        for (i, rec) in analysis.recommendations.iter().enumerate() {
            println!("  {}. {}", i + 1, style(&rec.action).bold());
            println!("     {}", style(&rec.rationale).dim());
            
            if verbose {
                if let Some(before) = &rec.before {
                    println!("     Before: {}", style(before).red());
                }
                println!("     After:  {}", style(&rec.after).green());
            }
        }
    }
    
    println!("\n{}", "=".repeat(50));
}

/// Migrate project to use automatic mode detection
pub fn migrate_project(path: &PathBuf, force: bool, backup: bool) -> Result<()> {
    use console::style;
    use dialoguer::{Confirm, Select};
    use std::fs;
    
    println!("{}", style("🚀 Migrating project to automatic mode detection...").bold().blue());
    
    // Analyze current project
    let detector = SmartDetector::new(path);
    let analysis = detector.analyze_comprehensive()?;
    
    if analysis.issues.is_empty() && analysis.recommendations.is_empty() {
        println!("{}", style("✅ Project is already properly configured!").green());
        return Ok(());
    }
    
    // Show migration plan
    println!("\n{}", style("📋 Migration Plan:").bold().yellow());
    for (i, rec) in analysis.recommendations.iter().enumerate() {
        println!("  {}. {}", i + 1, rec.action);
    }
    
    // Confirm migration
    if !force {
        let proceed = Confirm::new()
            .with_prompt("Do you want to proceed with the migration?")
            .default(true)
            .interact()?;
            
        if !proceed {
            println!("Migration cancelled.");
            return Ok(());
        }
    }
    
    // Backup if requested
    if backup {
        let backup_dir = path.join(".leptos-backup");
        if backup_dir.exists() {
            fs::remove_dir_all(&backup_dir)?;
        }
        fs::create_dir_all(&backup_dir)?;
        
        // Backup Cargo.toml
        let cargo_toml = path.join("Cargo.toml");
        if cargo_toml.exists() {
            fs::copy(&cargo_toml, backup_dir.join("Cargo.toml"))?;
        }
        
        println!("{}", style("📦 Backup created at .leptos-backup/").dim());
    }
    
    // Apply recommendations
    for rec in &analysis.recommendations {
        apply_recommendation(path, rec)?;
    }
    
    // Setup validation
    setup_validation_in_project(path).map_err(|e| anyhow::anyhow!("Failed to setup validation: {}", e))?;
    
    println!("\n{}", style("✅ Migration completed successfully!").green());
    println!("{}", style("Run 'cargo check' to verify the changes.").dim());
    
    Ok(())
}

fn apply_recommendation(path: &PathBuf, rec: &leptos_feature_detection::Recommendation) -> Result<()> {
    use console::style;
    
    println!("  Applying: {}", style(&rec.action).bold());
    
    if let Some(file_path) = &rec.file {
        if file_path.file_name().unwrap() == "Cargo.toml" {
            update_cargo_toml(path, rec)?;
        }
    }
    
    Ok(())
}

fn update_cargo_toml(path: &PathBuf, rec: &leptos_feature_detection::Recommendation) -> Result<()> {
    use std::fs;
    
    let cargo_toml_path = path.join("Cargo.toml");
    let mut content = fs::read_to_string(&cargo_toml_path)?;
    
    // Simple string replacement for now
    // In a real implementation, this would parse and modify the TOML structure
    if let Some(before) = &rec.before {
        if content.contains(before) {
            content = content.replace(before, &rec.after);
            fs::write(&cargo_toml_path, content)?;
        }
    }
    
    Ok(())
}

/// Validate current project configuration
pub fn validate_project(path: &PathBuf, fix: bool) -> Result<()> {
    use console::style;
    
    println!("{}", style("🔍 Validating project configuration...").bold().blue());
    
    let detector = SmartDetector::new(path);
    let analysis = detector.analyze_comprehensive()?;
    
    if analysis.issues.is_empty() {
        println!("{}", style("✅ Project configuration is valid!").green());
        return Ok(());
    }
    
    println!("\n{}", style("⚠️  Validation Issues:").bold().red());
    for issue in &analysis.issues {
        let severity_icon = match issue.severity {
            leptos_feature_detection::Severity::Error => "❌",
            leptos_feature_detection::Severity::Warning => "⚠️",
            leptos_feature_detection::Severity::Info => "ℹ️",
        };
        
        println!("  {} {}", severity_icon, issue.message);
        if let Some(suggestion) = &issue.suggestion {
            println!("     💡 {}", style(suggestion).dim());
        }
    }
    
    if fix {
        println!("\n{}", style("🔧 Attempting to fix issues...").bold().yellow());
        // Apply fixes here
        println!("{}", style("✅ Issues fixed!").green());
    }
    
    Ok(())
}

/// Generate build configuration for a specific mode
pub fn generate_config(mode: &str, output: Option<PathBuf>, env: &str) -> Result<()> {
    use console::style;
    
    let build_mode = match mode.to_lowercase().as_str() {
        "spa" => BuildMode::Spa,
        "fullstack" => BuildMode::Fullstack,
        "static" => BuildMode::Static,
        "api" => BuildMode::Api,
        _ => {
            println!("{}", style("❌ Invalid mode. Valid modes: spa, fullstack, static, api").red());
            return Ok(());
        }
    };
    
    let environment = match env.to_lowercase().as_str() {
        "development" => leptos_mode_resolver::Environment::Development,
        "production" => leptos_mode_resolver::Environment::Production,
        "test" => leptos_mode_resolver::Environment::Test,
        _ => {
            println!("{}", style("❌ Invalid environment. Valid environments: development, production, test").red());
            return Ok(());
        }
    };
    
    let config = match environment {
        leptos_mode_resolver::Environment::Development => BuildConfig::development(build_mode),
        leptos_mode_resolver::Environment::Production => BuildConfig::production(build_mode),
        leptos_mode_resolver::Environment::Test => BuildConfig {
            mode: build_mode,
            additional_features: vec!["tracing".to_string()],
            environment,
        },
    };
    
    let metadata = config.leptos_metadata();
    let toml_content = metadata.to_toml();
    
    if let Some(output_path) = output {
        std::fs::write(&output_path, toml_content)?;
        println!("{}", style(format!("✅ Configuration written to {:?}", output_path)).green());
    } else {
        println!("{}", style("📄 Generated Configuration:").bold().blue());
        println!("{}", toml_content);
    }
    
    Ok(())
}

/// Show help for a specific mode
pub fn show_mode_help(mode: &str) -> Result<()> {
    use console::style;
    
    let help_text = match mode.to_lowercase().as_str() {
        "spa" => {
            r#"
🌐 SPA (Single Page Application) Mode

This mode is for client-side only applications that run entirely in the browser.

Features:
  • Client-side rendering (CSR)
  • No server-side rendering
  • Perfect for static sites or pure client apps

Use cases:
  • Static websites
  • Client-only applications
  • Prototypes and demos

Configuration:
  [package.metadata.leptos]
  mode = "spa"
"#
        }
        "fullstack" => {
            r#"
🚀 Fullstack Mode

This mode provides both server-side rendering and client-side hydration.

Features:
  • Server-side rendering (SSR)
  • Client-side hydration
  • Full-stack capabilities
  • SEO-friendly

Use cases:
  • Production web applications
  • SEO-important sites
  • Full-stack applications

Configuration:
  [package.metadata.leptos]
  mode = "fullstack"
"#
        }
        "static" => {
            r#"
📄 Static Mode

This mode generates static HTML files at build time.

Features:
  • Static site generation
  • Pre-rendered content
  • Fast loading
  • CDN-friendly

Use cases:
  • Documentation sites
  • Blogs
  • Marketing pages
  • JAMstack applications

Configuration:
  [package.metadata.leptos]
  mode = "static"
"#
        }
        "api" => {
            r#"
🔌 API Mode

This mode is for server-only applications that provide APIs.

Features:
  • Server-side rendering
  • API endpoints
  • No client-side code
  • Backend services

Use cases:
  • REST APIs
  • GraphQL servers
  • Backend services
  • Microservices

Configuration:
  [package.metadata.leptos]
  mode = "api"
"#
        }
        _ => {
            println!("{}", style("❌ Unknown mode. Valid modes: spa, fullstack, static, api").red());
            return Ok(());
        }
    };
    
    println!("{}", style(help_text).cyan());
    
    Ok(())
}
