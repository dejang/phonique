mod crawler;
mod error;
pub mod models;
mod playlist;
pub mod ui;
use std::{collections::HashMap, time::SystemTime};

use log::{error, trace};
use models::{ArtistRelease, LabelRelease, Release, SearchParams, SearchResponse, SearchType};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DiscogsError {
    #[error("Deserialize error: {0}")]
    DeserializeError(String),
    #[error("Discogs API error: {0}")]
    DiscogsApiError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Authentication required")]
    AuthenticationRequired,
    #[error("Consumer key required")]
    ConsumerKeyRequired,
    #[error("Consumer secret required")]
    ConsumerSecretRequired,
}

impl From<reqwest::Error> for DiscogsError {
    fn from(error: reqwest::Error) -> Self {
        DiscogsError::NetworkError(error.to_string())
    }
}

impl From<serde_json::Error> for DiscogsError {
    fn from(error: serde_json::Error) -> Self {
        DiscogsError::DeserializeError(error.to_string())
    }
}

type Result<T> = std::result::Result<T, DiscogsError>;

#[derive(Debug)]
enum RequestMethod {
    GET,
    POST,
    PUT,
    DELETE,
}

const API_BASE_URL: &str = "https://api.discogs.com";

// TODO: fix configuring this when there is a UI section for
// configuring Discogs client.
#[derive(Default, Debug)]
pub struct DiscogsClient {
    client: reqwest::Client,
    consumer_key: String,
    consumer_secret: String,
    oauth_token: Option<String>,
    oauth_token_secret: Option<String>,
}

impl DiscogsClient {
    pub fn builder() -> DiscogsClientBuilder {
        DiscogsClientBuilder::new()
    }

    fn build_url(&self, path: &str) -> String {
        format!("{API_BASE_URL}{path}")
    }

    async fn request<T>(
        &self,
        path: &str,
        params: Option<HashMap<String, String>>,
        method: RequestMethod,
    ) -> Result<T>
    where
        T: DeserializeOwned + Debug,
    {
        if !self.is_auth() {
            return Err(DiscogsError::AuthenticationRequired);
        }

        let url = self.build_url(path);
        trace!("{method:?} request to {url:?}");

        let headers = self.build_request_headers();
        trace!("Sending headers {headers:?}");
        trace!("With request headers: {params:?}");

        let response = match method {
            RequestMethod::GET => {
                let resp = self
                    .client
                    .get(&url)
                    .headers(headers)
                    .query(&params.unwrap_or_default())
                    .send()
                    .await?;
                trace!("Received response with status: {}", resp.status());
                resp
            }
            RequestMethod::POST => todo!(),
            RequestMethod::PUT => todo!(),
            RequestMethod::DELETE => todo!(),
        };

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("API error: {error_text}");
            return Err(DiscogsError::DiscogsApiError(error_text));
        }

        let json_value = response.text().await?;
        trace!("Deserializing response...");

        let result = serde_json::from_str(&json_value)?;
        trace!("Deserialized response:\n{result:?}");

        Ok(result)
    }

    pub async fn artist_releases(&self, artist_id: &str) -> Result<Vec<ArtistRelease>> {
        let mut params = HashMap::new();
        params.insert("sort".to_string(), "year".to_string());
        params.insert("sort_order".to_string(), "desc".to_string());

        #[derive(Debug, serde::Deserialize)]
        struct ArtistReleasesResponse {
            releases: Vec<ArtistRelease>,
        }

        let response: ArtistReleasesResponse = self
            .request(
                &format!("/artists/{artist_id}/releases"),
                Some(params),
                RequestMethod::GET,
            )
            .await?;

        Ok(response.releases)
    }

    pub async fn release(&self, release_id: i32) -> Result<Release> {
        self.request(&format!("/releases/{release_id}"), None, RequestMethod::GET)
            .await
    }

    pub async fn label_releases(&self, label_id: &str) -> Result<Vec<LabelRelease>> {
        #[derive(Debug, serde::Deserialize)]
        struct LabelReleasesResponse {
            releases: Vec<LabelRelease>,
        }

        let response: LabelReleasesResponse = self
            .request(
                &format!("/labels/{label_id}/releases?per_page=200"),
                None,
                RequestMethod::GET,
            )
            .await?;

        Ok(response
            .releases
            .into_iter()
            .filter(|item| item.format.starts_with("12\"") || item.format.starts_with("7\""))
            .collect())
    }

    pub async fn search(&self, params: SearchParams) -> Result<SearchResponse> {
        let params = params
            .search_type(SearchType::Release)
            .page_size(200)
            .to_query_params();

        let response: SearchResponse = self
            .request("/database/search", Some(params), RequestMethod::GET)
            .await?;

        Ok(response)
    }

    fn is_auth(&self) -> bool {
        if self.oauth_token.is_none() || self.oauth_token_secret.is_none() {
            return false;
        }
        true
    }

    fn build_request_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Content-Type",
            HeaderValue::from_str("application/json").unwrap(),
        );

        let oauth_timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let oauth_nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let auth_header = format!(
            "OAuth oauth_consumer_key=\"{}\",oauth_token=\"{}\",oauth_signature=\"{}&{}\",oauth_signature_method=\"PLAINTEXT\",oauth_version=\"1.0\",oauth_timestamp=\"{}\",oauth_nonce=\"{}\"",
            self.consumer_key,
            self.oauth_token.as_ref().unwrap(),
            self.consumer_secret,
            self.oauth_token_secret.as_ref().unwrap(),
            oauth_timestamp,
            oauth_nonce,
        );

        headers.insert(
            "Authorization",
            HeaderValue::from_str(&auth_header).unwrap(),
        );
        headers.insert(
            "User-Agent",
            HeaderValue::from_str("Discogs Client").unwrap(),
        );

        headers
    }
}

pub struct DiscogsClientBuilder {
    consumer_key: Option<String>,
    consumer_secret: Option<String>,
    oauth_token: Option<String>,
    oauth_token_secret: Option<String>,
}

impl DiscogsClientBuilder {
    pub fn new() -> Self {
        Self {
            consumer_key: None,
            consumer_secret: None,
            oauth_token: None,
            oauth_token_secret: None,
        }
    }

    pub fn consumer_key(mut self, key: String) -> Self {
        self.consumer_key = Some(key);
        self
    }

    pub fn consumer_secret(mut self, secret: String) -> Self {
        self.consumer_secret = Some(secret);
        self
    }

    pub fn oauth_token(mut self, token: String) -> Self {
        self.oauth_token = Some(token);
        self
    }

    pub fn oauth_token_secret(mut self, secret: String) -> Self {
        self.oauth_token_secret = Some(secret);
        self
    }

    pub fn build(self) -> Result<DiscogsClient> {
        Ok(DiscogsClient {
            client: reqwest::Client::new(),
            consumer_key: self.consumer_key.ok_or(DiscogsError::ConsumerKeyRequired)?,
            consumer_secret: self
                .consumer_secret
                .ok_or(DiscogsError::ConsumerSecretRequired)?,
            oauth_token: self.oauth_token,
            oauth_token_secret: self.oauth_token_secret,
        })
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[tokio::test]
//     async fn test_get_release() {
//         let client = DiscogsClient::default();
//         let release = client
//             .release("29390533")
//             .await
//             .expect("Failed to get release");
//         assert!(
//             !release.tracklist.is_empty(),
//             "Tracklist should not be empty"
//         );
//     }

//     #[tokio::test]
//     async fn test_artist_releases() {
//         let client = DiscogsClient::default();
//         // Using Aphex Twin's artist ID as an example
//         let releases = client
//             .artist_releases("45")
//             .await
//             .expect("Failed to get artist releases");
//         assert!(!releases.is_empty(), "Artist should have releases");
//     }

//     #[tokio::test]
//     async fn test_label_releases() {
//         let client = DiscogsClient::default();
//         // Using Warp Records' label ID as an example
//         let releases = client
//             .label_releases("459")
//             .await
//             .expect("Failed to get label releases");
//         assert!(!releases.is_empty(), "Label should have releases");
//     }
// }
