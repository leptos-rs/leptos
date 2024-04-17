pub mod steps;
use anyhow::{anyhow, Result};

pub async fn wait() {
    tokio::time::sleep(tokio::time::Duration::from_millis(75)).await;
}

use regex::Regex;

fn extract_code_and_link(text: &str) -> Result<(String, String)> {
    // Regex pattern for a six-digit number
    let number_regex = Regex::new(r"\b\d{6}\b").unwrap();
    // Regex pattern for a URL
    let url_regex = Regex::new(r">(https?://[^<]+)<").unwrap(); // Simplified URL pattern

    // Search for a six-digit number
    let number = number_regex
        .find(text)
        .map(|match_| match_.as_str().to_string())
        .ok_or(anyhow!("Can't find number match"))?;

    // Search for a URL
    let url = url_regex
        .find(text)
        .map(|match_| match_.as_str().to_string())
        .ok_or(anyhow!("Can't find url match in \n {text}"))?;
    let url = url.trim_matches(|c| c == '>' || c == '<').to_string();
    let url = url.replace("amp;", "");
    Ok((number, url))
}

fn extract_code(text: &str) -> Result<String> {
    // Regex pattern for a six-digit number
    let number_regex = Regex::new(r"\b\d{6}\b").unwrap();

    // Search for a six-digit number
    let number = number_regex
        .find(text)
        .map(|match_| match_.as_str().to_string())
        .ok_or(anyhow!("Can't find number match"))?;
    Ok(number)
}
