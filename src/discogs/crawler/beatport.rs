use iced::futures;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use urlencoding::encode;

use crate::discogs::crawler::CrawlerId;

use super::{
    Stream,
    parser::Track,
    util::{Result, get_string_between},
};

#[derive(Serialize, Deserialize, Debug)]
struct Artist {
    artist_id: i32,
    artist_name: String,
    artist_type_name: String,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
struct SearchResultEntry {
    pub score: f32,
    pub add_date: Option<String>,
    pub artists: Vec<Artist>,
    pub available_worldwide: Option<i32>,
    pub bpm: Option<i32>,
    pub catalog_number: String,
    pub change_date: Option<String>,
    pub chord_type_id: Option<i32>,
    pub current_status: Option<Value>,
    pub enabled: Option<i32>,
    pub encode_status: Option<String>,
    pub exclusive_period: Option<i32>,
    pub genre_enabled: Option<i32>,
    pub guid: String,
    pub is_available_for_streaming: Option<i32>,
    pub is_classic: Option<i32>,
    pub isrc: Option<String>,
    pub key_id: Option<i32>,
    pub key_name: Option<String>,
    pub label: Value,
    pub label_manager: Option<String>,
    pub length: Option<i32>,
    pub mix_name: String,
    pub publish_date: Option<String>,
    pub publish_status: Option<String>,
    pub release: Value,
    pub release_date: Option<String>,
    pub sale_type: Option<String>,
    pub suggest: Value,
    pub supplier_id: Option<i32>,
    pub track_id: u64,
    pub track_name: String,
    pub track_number: i32,
    pub update_date: Option<String>,
    pub was_ever_exclusive: Option<i32>,
    pub downloads: Option<i32>,
    pub plays: Option<i32>,
    pub price: Value,
    pub is_explicit: Option<bool>,
    pub track_image_uri: Option<String>,
    pub track_image_dynamic_uri: Option<String>,
    pub genre: Value,
}

impl From<SearchResultEntry> for Track {
    fn from(value: SearchResultEntry) -> Self {
        log::debug!("{:?}", serde_json::to_string_pretty(&value));
        let artists = value
            .artists
            .iter()
            .filter(|a| a.artist_type_name.eq_ignore_ascii_case("artist"))
            .map(|a| a.artist_name.clone())
            .collect::<Vec<String>>();

        let remix = value
            .artists
            .iter()
            .filter(|a| a.artist_type_name.eq_ignore_ascii_case("remixer"))
            .map(|a| a.artist_name.clone())
            .collect::<Vec<String>>();

        let track_name = if value.track_name.contains(" - ") {
            value.track_name.split(" - ").last().unwrap().to_string()
        } else {
            value.track_name.clone()
        };
        Track {
            name: track_name,
            artists,
            catno: value.catalog_number,
            remix,
            position: value.track_number.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Beatport;

impl Beatport {
    fn get_search_url(&self, artist_names: &str, remixer_name: &str, track_name: &str) -> String {
        let encoded = encode(&format!("{artist_names} {remixer_name} {track_name}")).to_string();
        format!("https://www.beatport.com/search/tracks?q={encoded}&per-page=25")
    }

    pub async fn track_search(&self, track: &Track) -> Result<Option<Stream>> {
        log::debug!("Track search: {track:?}");
        let client = reqwest::Client::new();
        let response = client
            .get(self.get_search_url(
                &track.artist_name(None),
                &track.remixer_name(None),
                &track.name,
            ))
            .send()
            .await?;
        let body = response.text().await?;
        let json = get_string_between(
            &body,
            "<script id=\"__NEXT_DATA__\" type=\"application/json\">",
            "</script>",
        );

        let deserialized: Value = serde_json::from_str(&json)?;

        let results: Vec<Value> = deserialized
            .get("props")
            .unwrap()
            .get("pageProps")
            .unwrap()
            .get("dehydratedState")
            .unwrap()
            .get("queries")
            .unwrap()
            .as_array()
            .unwrap()
            .first()
            .unwrap()
            .get("state")
            .unwrap()
            .get("data")
            .unwrap()
            .get("data")
            .unwrap()
            .as_array()
            .unwrap()
            .to_owned();

        for v in results {
            let t: SearchResultEntry = serde_json::from_value(v)?;
            let track_id = t.track_id;
            let track_guid = t.guid.clone();
            let maybe_track = Track::from(t);
            log::trace!("Check if {track:?} matches with {maybe_track:?}");
            if track.eq(&maybe_track) {
                let slug = maybe_track
                    .name
                    .to_lowercase()
                    .replace(" ", "-")
                    .to_string();
                log::trace!("Track matches! Returning.");
                return Ok(Some(Stream {
                    page_url: format!("https://www.beatport.com/track/{slug}/{track_id}"),
                    audio_url: format!(
                        "https://geo-samples.beatport.com/track/{track_guid}.LOFI.mp3"
                    ),
                    craler_id: CrawlerId::Beatport,
                }));
            }
        }
        Ok(None)
    }
    pub async fn album_search(&self, tracks: &[Track]) -> Vec<Result<Option<Stream>>> {
        let results = tracks.iter().map(|t| self.track_search(t));
        futures::future::join_all(results).await
    }
}

#[cfg(test)]
mod tests {
    use crate::discogs::crawler::parser::Track;

    use super::Beatport;

    #[tokio::test]
    async fn test_must_find_1() {
        let json = r#"{"artists":["Egal 3"],"catno":"MEM011","name":"Play You (Povestea Continua Mix)","position":"B2","remix":[],"url":null}"#;
        let track: Track = serde_json::from_str(json).unwrap();
        let crawler = Beatport;
        let results = crawler.track_search(&track).await;
        assert!(results.is_ok());
        if let Ok(stream) = results {
            println!("{stream:?}");
        }
    }

    #[tokio::test]
    async fn test_must_find_2() {
        let json = r#"{"name":"Bellon's Utopian Dreams","artists":["Gorbani & Enzo Leep"],"catno":"MNV002","remix":[],"url":null,"position":"A1"}"#;
        let track: Track = serde_json::from_str(json).unwrap();
        let crawler = Beatport;
        let results = crawler.track_search(&track).await;
        assert!(results.is_ok());
        if let Ok(stream) = results {
            println!("{stream:?}");
        }
    }

    #[tokio::test]
    async fn test_must_find_3() {
        let json = r#"{"name":"L.E.M","artists":["Gorbani & Enzo Leep"],"catno":"MNV002","remix":[],"url":null,"position":"B1"}"#;
        let track: Track = serde_json::from_str(json).unwrap();
        let crawler = Beatport;
        let results = crawler.track_search(&track).await;
        assert!(results.is_ok());
        if let Ok(stream) = results {
            println!("{stream:?}");
        }
    }
}
