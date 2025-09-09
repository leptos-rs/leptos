//! Utility functions for the CLI tool

use anyhow::Result;
use std::path::PathBuf;

/// Check if a path is a valid Leptos project
pub fn is_leptos_project(path: &PathBuf) -> bool {
    let cargo_toml = path.join("Cargo.toml");
    if !cargo_toml.exists() {
        return false;
    }
    
    // Check if Cargo.toml contains leptos dependency
    if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
        content.contains("leptos")
    } else {
        false
    }
}

/// Find the project root directory
pub fn find_project_root(start_path: &PathBuf) -> Result<PathBuf> {
    let mut current = start_path.clone();
    
    loop {
        if is_leptos_project(&current) {
            return Ok(current);
        }
        
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }
    
    Err(anyhow::anyhow!("No Leptos project found"))
}

/// Get project name from Cargo.toml
pub fn get_project_name(path: &PathBuf) -> Result<String> {
    let cargo_toml = path.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml)?;
    
    for line in content.lines() {
        if line.trim().starts_with("name = ") {
            if let Some(name) = line.split('"').nth(1) {
                return Ok(name.to_string());
            }
        }
    }
    
    Err(anyhow::anyhow!("Could not find project name in Cargo.toml"))
}

/// Format file size in human readable format
pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.1} {}", size, UNITS[unit_index])
}

/// Format duration in human readable format
pub fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    
    if secs > 0 {
        format!("{}.{:03}s", secs, millis)
    } else {
        format!("{}ms", millis)
    }
}

/// Create a progress bar with custom style
pub fn create_progress_bar(total: u64) -> indicatif::ProgressBar {
    use indicatif::{ProgressBar, ProgressStyle};
    
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-")
    );
    
    pb
}

/// Print a colored banner
pub fn print_banner(text: &str) {
    use console::style;
    
    let banner = format!(" {} ", text);
    let border = "=".repeat(banner.len());
    
    println!("{}", style(&border).bold().blue());
    println!("{}", style(&banner).bold().white().on_blue());
    println!("{}", style(&border).bold().blue());
}

/// Print a success message
pub fn print_success(message: &str) {
    use console::style;
    println!("{}", style(format!("✅ {}", message)).green());
}

/// Print a warning message
pub fn print_warning(message: &str) {
    use console::style;
    println!("{}", style(format!("⚠️  {}", message)).yellow());
}

/// Print an error message
pub fn print_error(message: &str) {
    use console::style;
    println!("{}", style(format!("❌ {}", message)).red());
}

/// Print an info message
pub fn print_info(message: &str) {
    use console::style;
    println!("{}", style(format!("ℹ️  {}", message)).blue());
}

/// Confirm action with user
pub fn confirm_action(prompt: &str, default: bool) -> Result<bool> {
    use dialoguer::Confirm;
    
    Ok(Confirm::new()
        .with_prompt(prompt)
        .default(default)
        .interact()?)
}

/// Select from a list of options
pub fn select_option<T: std::fmt::Display>(prompt: &str, items: &[T]) -> Result<usize> {
    use dialoguer::Select;
    
    Ok(Select::new()
        .with_prompt(prompt)
        .items(items)
        .interact()?)
}

/// Input text from user
pub fn input_text(prompt: &str, default: Option<&str>) -> Result<String> {
    use dialoguer::Input;
    
    let mut input = Input::<String>::new().with_prompt(prompt);
    if let Some(default_value) = default {
        input = input.default(default_value.to_string());
    }
    
    Ok(input.interact()?)
}

/// Check if running in CI environment
pub fn is_ci() -> bool {
    std::env::var("CI").is_ok() || 
    std::env::var("GITHUB_ACTIONS").is_ok() ||
    std::env::var("GITLAB_CI").is_ok() ||
    std::env::var("JENKINS_URL").is_ok()
}

/// Get terminal width for formatting
pub fn get_terminal_width() -> usize {
    use console::Term;
    
    Term::stdout().size().1 as usize
}

/// Wrap text to terminal width
pub fn wrap_text(text: &str, width: Option<usize>) -> String {
    let width = width.unwrap_or_else(get_terminal_width);
    let mut result = String::new();
    let mut current_line = String::new();
    
    for word in text.split_whitespace() {
        if current_line.len() + word.len() + 1 > width {
            if !current_line.is_empty() {
                result.push_str(&current_line);
                result.push('\n');
                current_line.clear();
            }
        }
        
        if !current_line.is_empty() {
            current_line.push(' ');
        }
        current_line.push_str(word);
    }
    
    if !current_line.is_empty() {
        result.push_str(&current_line);
    }
    
    result
}
