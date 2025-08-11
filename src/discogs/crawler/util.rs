use regex::Regex;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, CrawlerError>;

#[derive(Debug, Error)]
pub enum CrawlerError {
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("SerdeError error: {0}")]
    SerdeError(#[from] serde_json::Error),
}

pub fn get_string_between(input: &str, left: &str, right: &str) -> String {
    let parts: Vec<&str> = input.split(left).collect();
    let parts: Vec<&str> = parts[1].split(right).collect();
    parts[0].to_string()
}

pub fn strip_artist_name_chars(input: &str) -> String {
    let multispace_matcher = Regex::new(r#"\s{2,}"#).unwrap();
    let input = input.replace("&", "").replace(",", "");
    let input = multispace_matcher.replace_all(&input, " ").to_string();
    input.trim().to_string()
}
