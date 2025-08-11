mod bandcamp;
mod beatport;
pub mod parser;
mod util;

use bandcamp::Bandcamp;
use beatport::Beatport;
use iced::futures;
use parser::{Track, parse_track};
use util::{Result, strip_artist_name_chars};

use super::models::Release;

#[derive(Debug, Clone)]
pub enum CrawlerId {
    Beatport,
    Bandcamp,
}

impl std::fmt::Display for CrawlerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CrawlerId::Beatport => f.write_str("BT"),
            CrawlerId::Bandcamp => f.write_str("BC"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Stream {
    pub page_url: String,
    pub audio_url: String,
    pub craler_id: CrawlerId,
}

pub async fn crawl_release(release: Release) -> Result<Vec<Vec<Option<Stream>>>> {
    let tracks: Vec<Track> = release
        .tracklist
        .iter()
        .map(|t| {
            log::trace!(
                "Track Artists: {:?}\nTrack Extraartists: {:?}",
                t.artists,
                t.extraartists
            );
            Track {
                name: parse_track(&format!("a - {}", t.title)).name,
                artists: if let Some(artists) = t.artists_clean() {
                    artists
                } else {
                    release
                        .artists
                        .iter()
                        .map(|a| strip_artist_name_chars(&a.name))
                        .collect()
                },
                remix: t.remixers_clean().unwrap_or_default(),
                catno: release.cat_no(),
                position: t.position.clone(),
            }
        })
        .collect();
    let beatport_future = Beatport.album_search(&tracks);
    let bandcamp_future = Bandcamp.album_search(&tracks);

    let (bandcamp, beatport) = futures::future::join(bandcamp_future, beatport_future).await;
    let mut out: Vec<Vec<Option<Stream>>> = vec![vec![]; tracks.len()];
    for s in [beatport, bandcamp] {
        out.iter_mut().zip(s.into_iter()).for_each(|(t, s)| {
            if let Ok(stream) = s {
                t.push(stream);
            } else if let Err(error) = s {
                log::error!("Error crawling: {error}");
            }
        });
    }
    Ok(out)
}
