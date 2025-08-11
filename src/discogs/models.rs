use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Artist {
    pub anv: String,
    pub id: i32,
    pub join: String,
    pub name: String,
    pub resource_url: String,
    pub role: String,
    pub tracks: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Contributor {
    pub resource_url: String,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Company {
    pub catno: String,
    pub entity_type: String,
    pub entity_type_name: String,
    pub id: i32,
    pub name: String,
    pub resource_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Rating {
    pub average: Option<f32>,
    pub count: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Format {
    pub descriptions: Option<Vec<String>>,
    pub name: String,
    pub qty: String,
    pub text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Community {
    pub contributors: Vec<Option<Contributor>>,
    pub data_quality: String,
    pub have: i32,
    pub rating: Rating,
    pub status: String,
    pub submitter: Option<Contributor>,
    pub want: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Identifier {
    #[serde(rename = "type")]
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Image {
    pub height: i32,
    pub resource_url: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub uri: String,
    pub uri150: String,
    pub width: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Label {
    pub catno: String,
    pub entity_type: String,
    pub id: i32,
    pub name: String,
    pub resource_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Track {
    pub duration: String,
    pub position: String,
    pub title: String,
    pub type_: String,
    pub artists: Option<Vec<Artist>>,
    pub extraartists: Option<Vec<Artist>>,
}

impl Track {
    pub fn artist_or_release_artist(&self, release_artist: &str) -> String {
        if let Some(artists) = &self.artists {
            artists
                .iter()
                .map(|a| a.name.clone())
                .collect::<Vec<String>>()
                .join(", ")
        } else {
            release_artist.to_string()
        }
    }

    pub fn artists_clean(&self) -> Option<Vec<String>> {
        if let Some(artists) = &self.artists {
            let pattern = regex::Regex::new(r"\([0-9]+\)").unwrap();
            return Some(
                artists
                    .iter()
                    .map(|a| pattern.replace(&a.name, "").trim().to_string())
                    .collect::<Vec<String>>(),
            );
        }
        None
    }

    pub fn remixers_clean(&self) -> Option<Vec<String>> {
        if let Some(artists) = &self.extraartists {
            let pattern = regex::Regex::new(r"\([0-9]+\)").unwrap();
            return Some(
                artists
                    .iter()
                    .filter(|a| a.role.to_lowercase().contains("remix"))
                    .map(|a| pattern.replace(&a.name, "").trim().to_string())
                    .collect::<Vec<String>>(),
            );
        }
        None
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Video {
    pub description: Option<String>,
    pub duration: i32,
    pub embed: bool,
    pub title: String,
    pub uri: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Release {
    pub title: String,
    pub id: i32,
    pub artists: Vec<Artist>,
    pub data_quality: String,
    pub thumb: Option<String>,
    pub community: Community,
    pub companies: Vec<Company>,
    pub country: Option<String>,
    pub date_added: String,
    pub date_changed: String,
    pub estimated_weight: Option<i32>,
    pub extraartists: Option<Vec<Artist>>,
    pub format_quantity: Option<i32>,
    pub formats: Vec<Format>,
    pub genres: Vec<String>,
    pub identifiers: Vec<Identifier>,
    pub images: Option<Vec<Image>>,
    pub labels: Vec<Label>,
    pub lowest_price: Option<f32>,
    pub master_id: Option<i32>,
    pub master_url: Option<String>,
    pub notes: Option<String>,
    pub num_for_sale: Option<i32>,
    pub released: Option<String>,
    pub release_formatted: Option<String>,
    pub resource_url: String,
    pub series: Option<Value>,
    pub status: String,
    pub styles: Option<Vec<String>>,
    pub tracklist: Vec<Track>,
    pub uri: String,
    pub videos: Option<Vec<Video>>,
    pub year: Option<i32>,
}

impl Release {
    pub fn artist(&self) -> String {
        let pattern = regex::Regex::new(r"\([0-9]+\)").unwrap();
        self.artists
            .iter()
            .map(|a| pattern.replace(&a.name, "").to_string())
            .collect::<Vec<String>>()
            .join(",")
            .trim()
            .to_string()
    }

    pub fn label(&self) -> Label {
        self.labels.first().unwrap().clone()
    }

    pub fn cat_no(&self) -> String {
        self.labels.first().unwrap().catno.clone()
    }

    pub fn tracks(&self) -> Vec<String> {
        self.tracklist
            .iter()
            .map(|t| t.title.clone())
            .collect::<Vec<String>>()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pagination {
    pub per_page: i32,
    pub items: i32,
    pub page: i32,
    pub urls: Value,
    pub pages: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArtistRelease {
    pub artist: String,
    pub id: i32,
    pub main_release: Option<i32>,
    pub resource_url: String,
    pub role: Option<String>,
    pub thumb: Option<String>,
    pub title: String,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub year: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArtistReleases {
    pub pagination: Pagination,
    pub releases: Vec<ArtistRelease>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LabelRelease {
    pub artist: String,
    pub catno: String,
    pub format: String,
    pub id: i32,
    pub resource_url: String,
    pub status: String,
    pub thumb: Option<String>,
    pub title: String,
    pub year: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LabelReleases {
    pub pagination: Pagination,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserIdentity {
    pub id: i32,
    pub username: String,
    pub resource_url: String,
    pub consumer_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WantBasicInformation {
    pub formats: Vec<Format>,
    pub thumb: Option<String>,
    pub cover_image: Option<String>,
    pub title: String,
    pub labels: Vec<Label>,
    pub year: i32,
    pub artists: Vec<Artist>,
    pub resource_url: String,
    pub id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Want {
    pub rating: i32,
    pub basic_information: WantBasicInformation,
    pub resource_url: String,
    pub id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WantList {
    pagination: Pagination,
    wants: Vec<Want>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SellerRelease {
    catalog_number: Option<String>,
    resource_url: String,
    year: i32,
    id: i32,
    descriptiont: Option<String>,
    artist: String,
    title: String,
    format: Option<String>,
    thumbnail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SellerListing {
    status: String,
    price: Value,
    allow_offers: bool,
    sleeve_condition: Option<String>,
    id: i32,
    posted: Value,
    ships_from: String,
    uri: String,
    comments: Option<String>,
    seller: Value,
    release: SellerRelease,
    resource_url: String,
    audio: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SellerListings {
    pagination: Pagination,
    listings: Vec<SellerListing>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResultCommunity {
    pub want: i32,
    pub have: i32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub style: Vec<String>,
    pub thumb: Option<String>,
    pub title: String,
    pub country: Option<String>,
    pub format: Vec<String>,
    pub uri: String,
    pub community: SearchResultCommunity,
    pub label: Vec<String>,
    pub catno: String,
    pub year: Option<String>,
    pub genre: Vec<String>,
    pub resource_url: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub pagination: Pagination,
    pub results: Vec<SearchResult>,
}

pub enum SearchType {
    Release,
    Master,
    Artist,
    Label,
}

impl std::fmt::Display for SearchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchType::Release => f.write_str("release"),
            SearchType::Master => f.write_str("master"),
            SearchType::Artist => f.write_str("artist"),
            SearchType::Label => f.write_str("label"),
        }
    }
}

#[derive(Default)]
pub struct SearchParams {
    pub query: Option<String>,
    pub search_type: Option<SearchType>,
    pub genre: Option<String>,
    pub style: Option<String>,
    pub track: Option<String>,
    pub country: Option<String>,
    pub year: Option<u16>,
    pub catno: Option<String>,
    pub barcode: Option<String>,
    pub label: Option<String>,
    pub artist: Option<String>,
    pub page_size: Option<u8>,
}

impl SearchParams {
    pub fn query(mut self, value: String) -> Self {
        self.query = Some(value);
        self
    }

    pub fn search_type(mut self, value: SearchType) -> Self {
        self.search_type = Some(value);
        self
    }

    pub fn genre(mut self, value: String) -> Self {
        self.genre = Some(value);
        self
    }

    pub fn style(mut self, value: String) -> Self {
        self.style = Some(value);
        self
    }

    pub fn track(mut self, value: String) -> Self {
        self.track = Some(value);
        self
    }

    pub fn country(mut self, value: String) -> Self {
        self.country = Some(value);
        self
    }

    pub fn year(mut self, value: u16) -> Self {
        self.year = Some(value);
        self
    }

    pub fn catno(mut self, value: String) -> Self {
        self.catno = Some(value);
        self
    }

    pub fn barcode(mut self, value: String) -> Self {
        self.barcode = Some(value);
        self
    }

    pub fn label(mut self, value: String) -> Self {
        self.label = Some(value);
        self
    }

    pub fn artist(mut self, value: String) -> Self {
        self.artist = Some(value);
        self
    }

    pub fn page_size(mut self, value: u8) -> Self {
        self.page_size = Some(value);
        self
    }

    pub fn to_query_params<'a>(&'a self) -> HashMap<String, String> {
        let mut params: HashMap<String, String> = HashMap::new();
        if let Some(query) = &self.query {
            params.insert("q".into(), query.to_string());
        }

        if let Some(kind) = &self.search_type {
            params.insert("type".into(), kind.to_string());
        }

        if let Some(genre) = &self.genre {
            params.insert("genre".into(), genre.to_string());
        }

        if let Some(style) = &self.style {
            params.insert("style".into(), style.to_string());
        }

        if let Some(track) = &self.track {
            params.insert("track".into(), track.to_string());
        }

        if let Some(country) = &self.country {
            params.insert("country".into(), country.to_string());
        }

        if let Some(year) = &self.year {
            params.insert("year".into(), year.to_string());
        }

        if let Some(catno) = &self.catno {
            params.insert("catno".into(), catno.to_string());
        }

        if let Some(barcode) = &self.barcode {
            params.insert("barcode".into(), barcode.to_string());
        }

        if let Some(label) = &self.label {
            params.insert("label".into(), label.to_string());
        }

        if let Some(artist) = &self.artist {
            params.insert("artist".into(), artist.to_string());
        }

        if let Some(page_size) = &self.page_size {
            params.insert("per_page".into(), page_size.to_string());
        }

        params
    }
}
