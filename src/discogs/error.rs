use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct DiscogsClientError {
    pub source: Option<&'static (dyn Error + 'static)>,
    pub message: String,
}

impl Display for DiscogsClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}\n\n{:?}", self.message, self.source))
    }
}

impl Error for DiscogsClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

impl From<reqwest::Error> for DiscogsClientError {
    fn from(value: reqwest::Error) -> Self {
        Self {
            message: value.to_string(),
            source: None,
        }
    }
}

impl From<&str> for DiscogsClientError {
    fn from(value: &str) -> Self {
        Self {
            message: value.into(),
            source: None,
        }
    }
}
