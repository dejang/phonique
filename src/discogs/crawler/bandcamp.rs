use super::{parser::Track, util::Result};
use iced::futures;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use super::{Stream, parser::parse_track};

#[allow(dead_code)]
#[derive(Deserialize)]
struct SuggestionResultEntry {
    pub r#type: String,
    pub id: Option<i64>,
    pub art_id: Option<i64>,
    pub img_id: Option<i64>,
    pub name: String,
    pub band_id: Option<i64>,
    pub band_name: String,
    pub album_name: Option<String>,
    pub item_url_root: String,
    pub item_url_path: String,
    pub img: String,
    pub album_id: Option<i64>,
    pub stat_params: String,
}

#[derive(Deserialize)]
struct SuggestionResults {
    pub results: Vec<SuggestionResultEntry>,
}

#[derive(Deserialize)]
struct Suggestions {
    pub auto: SuggestionResults,
}

#[derive(Debug, Clone)]
pub struct Bandcamp;

impl Bandcamp {
    async fn get_suggestions(
        &self,
        artist_name: &str,
        track_name: &str,
    ) -> Result<Vec<SuggestionResultEntry>> {
        let json = Client::default()
            .post("https://bandcamp.com/api/bcsearch_public_api/1/autocomplete_elastic")
            .header("Content-Type", "application/json")
            .body(
                json!({
                    "fan_id": null,
                    "full_page": false,
                    "search_filter": "t",
                    "search_text": format!("{} {}", artist_name, track_name)
                })
                .to_string(),
            )
            .send()
            .await?
            .text()
            .await?;

        let suggestions: Suggestions = serde_json::from_str(&json)?;

        Ok(suggestions.auto.results)
    }

    pub async fn track_search(&self, track: &Track) -> Result<Option<Stream>> {
        let client = reqwest::Client::new();
        let results = self
            .get_suggestions(&track.artist_name(None), &track.name)
            .await?;

        let mut found = None;
        for entry in results {
            let entry_track = parse_track(&format!("{} - {}", entry.band_name, entry.name));
            if entry_track.eq(track) {
                found = Some(entry);
                break;
            }
        }

        if let Some(found) = found {
            let item_url = found.item_url_path;
            let track_page = client.get(&item_url).send().await?.text().await?;
            let player_matcher = Regex::new(r#"data-tralbum="(?P<json_data>.+?)""#).unwrap();
            let json_string = player_matcher
                .captures(&track_page)
                .unwrap()
                .name("json_data")
                .unwrap()
                .as_str()
                .replace("&quot;", "\"");

            let json: serde_json::Value = serde_json::from_str(&json_string)?;
            if let Some(tracklist) = json.get("trackinfo") {
                let tracklist = tracklist.as_array().unwrap();
                let json_artist_name = json.get("artist").unwrap().as_str().unwrap();
                for t in tracklist {
                    let track_name = t.get("title").unwrap().as_str().unwrap();
                    let bandcamp_track = parse_track(&format!("{json_artist_name} - {track_name}"));
                    if track.eq(&bandcamp_track)
                        && let Some(streams) = t.get("file")
                        && let Some(stream_url) = streams.get("mp3-128")
                    {
                        return Ok(Some(Stream {
                            page_url: item_url,
                            audio_url: stream_url.to_string().replace("\"", ""),
                            craler_id: super::CrawlerId::Bandcamp,
                        }));
                    }
                }
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
mod test {
    use crate::discogs::crawler::parser::Track;

    use super::Bandcamp;

    #[tokio::test]
    async fn must_find_1() {
        let results = Bandcamp
            .track_search(&Track::from_str("Barac - A story behind everything"))
            .await;
        assert!(results.is_ok());
        if let Ok(stream) = results {
            println!("{stream:?}");
        }
    }
}
