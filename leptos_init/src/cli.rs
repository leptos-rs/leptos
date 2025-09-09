//! CLI integration for enhanced leptos init command

use crate::{Database, InitConfig, ProjectGenerator, ProjectTemplate, ServerBackend, Styling};
use std::io::{self, Write};
use std::path::Path;

/// CLI interface for leptos init command
pub struct LeptosInitCli;

impl LeptosInitCli {
    /// Run the interactive init wizard
    pub fn run_interactive(project_name: String, target_path: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Leptos Enhanced Project Initialization");
        println!("Creating a new Leptos project: {}", project_name);
        println!();

        let template = Self::select_template()?;
        let config = Self::configure_project(project_name, template)?;
        
        println!("\n📋 Project Configuration:");
        Self::display_config(&config);
        
        if Self::confirm_creation()? {
            let generator = ProjectGenerator::new(config, target_path);
            generator.generate()?;
            
            println!("\n✅ Project created successfully!");
            Self::display_next_steps();
        } else {
            println!("❌ Project creation cancelled.");
        }

        Ok(())
    }

    /// Quick project creation with template
    pub fn run_with_template(
        project_name: String, 
        template: ProjectTemplate,
        target_path: impl AsRef<Path>
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Creating Leptos {} project: {}", 
            match template {
                ProjectTemplate::Spa => "SPA",
                ProjectTemplate::Fullstack => "fullstack", 
                ProjectTemplate::Static => "static",
                ProjectTemplate::Api => "API",
                ProjectTemplate::Custom => "custom",
            },
            project_name
        );

        let config = InitConfig::for_template(project_name, template);
        let generator = ProjectGenerator::new(config, target_path);
        generator.generate()?;

        println!("✅ Project created successfully!");
        Self::display_next_steps();

        Ok(())
    }

    fn select_template() -> Result<ProjectTemplate, Box<dyn std::error::Error>> {
        println!("📋 Select project template:");
        println!("1. Fullstack (SSR + Client hydration) - Recommended");
        println!("2. SPA (Client-side only)");
        println!("3. Static (Static site generation)");
        println!("4. API (Server functions only)");
        println!("5. Custom (Interactive configuration)");
        
        loop {
            print!("\nEnter choice (1-5): ");
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            
            match input.trim() {
                "1" => return Ok(ProjectTemplate::Fullstack),
                "2" => return Ok(ProjectTemplate::Spa),
                "3" => return Ok(ProjectTemplate::Static),
                "4" => return Ok(ProjectTemplate::Api),
                "5" => return Ok(ProjectTemplate::Custom),
                _ => println!("❌ Invalid choice. Please enter 1-5."),
            }
        }
    }

    fn configure_project(project_name: String, template: ProjectTemplate) -> Result<InitConfig, Box<dyn std::error::Error>> {
        let mut config = InitConfig::for_template(project_name, template.clone());
        
        if template == ProjectTemplate::Custom {
            config = Self::run_custom_wizard(config)?;
        } else if template != ProjectTemplate::Spa && template != ProjectTemplate::Api {
            // Ask about optional features for fullstack/static
            config.server = Self::select_server_backend()?;
            
            if Self::ask_yes_no("Add database support?")? {
                config.database = Self::select_database()?;
            }
            
            if Self::ask_yes_no("Add Tailwind CSS?")? {
                config.styling = Styling::Tailwind;
            }
        }

        Ok(config)
    }

    fn run_custom_wizard(mut config: InitConfig) -> Result<InitConfig, Box<dyn std::error::Error>> {
        println!("\n🔧 Custom Configuration Wizard");
        
        // Select rendering mode
        println!("\nRendering mode:");
        println!("1. Client-side rendering (SPA)");
        println!("2. Server-side rendering (SSR)");  
        println!("3. Static site generation");
        
        loop {
            print!("Select rendering mode (1-3): ");
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            
            match input.trim() {
                "1" => {
                    config.template = ProjectTemplate::Spa;
                    break;
                }
                "2" => {
                    config.template = ProjectTemplate::Fullstack;
                    config.server = Self::select_server_backend()?;
                    break;
                }
                "3" => {
                    config.template = ProjectTemplate::Static;
                    break;
                }
                _ => println!("❌ Invalid choice. Please enter 1-3."),
            }
        }

        // Additional features
        if config.template != ProjectTemplate::Spa {
            if Self::ask_yes_no("\nAdd database support?")? {
                config.database = Self::select_database()?;
            }
        }

        if Self::ask_yes_no("Add Tailwind CSS?")? {
            config.styling = Styling::Tailwind;
        }

        if Self::ask_yes_no("Enable tracing?")? {
            config.use_tracing = true;
        }

        if Self::ask_yes_no("Enable islands architecture?")? {
            config.use_islands = true;
        }

        Ok(config)
    }

    fn select_server_backend() -> Result<ServerBackend, Box<dyn std::error::Error>> {
        println!("\nServer backend:");
        println!("1. Axum (Recommended)");
        println!("2. Actix Web");
        println!("3. Warp");
        
        loop {
            print!("Select backend (1-3): ");
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            
            match input.trim() {
                "1" => return Ok(ServerBackend::Axum),
                "2" => return Ok(ServerBackend::Actix),
                "3" => return Ok(ServerBackend::Warp),
                _ => println!("❌ Invalid choice. Please enter 1-3."),
            }
        }
    }

    fn select_database() -> Result<Database, Box<dyn std::error::Error>> {
        println!("\nDatabase:");
        println!("1. SQLite (Recommended)");
        println!("2. PostgreSQL");
        println!("3. MySQL");
        
        loop {
            print!("Select database (1-3): ");
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            
            match input.trim() {
                "1" => return Ok(Database::Sqlite),
                "2" => return Ok(Database::Postgresql),
                "3" => return Ok(Database::Mysql),
                _ => println!("❌ Invalid choice. Please enter 1-3."),
            }
        }
    }

    fn ask_yes_no(question: &str) -> Result<bool, Box<dyn std::error::Error>> {
        loop {
            print!("{} (y/n): ", question);
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            
            match input.trim().to_lowercase().as_str() {
                "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                _ => println!("❌ Please enter 'y' or 'n'."),
            }
        }
    }

    fn display_config(config: &InitConfig) {
        println!("  📦 Name: {}", config.name);
        println!("  🏗️  Template: {:?}", config.template);
        if config.template != ProjectTemplate::Spa && config.template != ProjectTemplate::Api {
            println!("  🖥️  Server: {:?}", config.server);
        }
        if config.database != Database::None {
            println!("  🗄️  Database: {:?}", config.database);
        }
        if config.styling != Styling::None {
            println!("  🎨 Styling: {:?}", config.styling);
        }
        if config.use_tracing {
            println!("  📊 Tracing: Enabled");
        }
        if config.use_islands {
            println!("  🏝️  Islands: Enabled");
        }
    }

    fn confirm_creation() -> Result<bool, Box<dyn std::error::Error>> {
        Self::ask_yes_no("\n🚀 Create project with this configuration?")
    }

    fn display_next_steps() {
        println!("\n📖 Next Steps:");
        println!("  1. cd into your project directory");
        println!("  2. Run `cargo leptos watch` to start development");
        println!("  3. Open http://127.0.0.1:3000 in your browser");
        println!("\n📚 Resources:");
        println!("  • Documentation: https://leptos.dev");
        println!("  • Book: https://leptos-rs.github.io/leptos/");
        println!("  • Discord: https://discord.gg/YdRAhS7eQB");
        println!("\n🎉 Happy coding with Leptos!");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_quick_template_creation() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path().join("test-project");
        
        let result = LeptosInitCli::run_with_template(
            "test-project".to_string(),
            ProjectTemplate::Spa,
            &project_path
        );
        
        // Should succeed without user interaction
        assert!(result.is_ok());
        
        // Verify project was created
        assert!(project_path.join("Cargo.toml").exists());
        assert!(project_path.join("src/main.rs").exists());
        assert!(project_path.join("README.md").exists());
    }
}