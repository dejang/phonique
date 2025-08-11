use std::{error::Error, path::PathBuf};

use lofty::{file::{AudioFile, TaggedFileExt}, tag::Accessor};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScannedKind {
    LocalFile = 0,
    GoogleDrive = 1,
    Dropbox = 2,
    Youtube = 3,
    Stream = 4,
}

#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: u16,
    pub genre: String,
    pub duration: u64,
    pub path: String,
    pub artwork: Option<Vec<u8>>,
    pub kind: ScannedKind,
}

pub fn scan_file(path: &PathBuf) -> Result<ScannedFile, Box<dyn Error>> {
    if !path.exists() {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Not Found",
        ))?;
    }

    // Read metadata using lofty
    let tagged_file = lofty::read_from_path(path)?;

    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag());
    let duration = tagged_file.properties().duration();
    let pictures = tag.as_ref().map(|t| t.pictures()).unwrap_or_default();
    let cover_art = pictures.first().map(|pic| {
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut pic.data(), &mut data).unwrap();
        data
    });

    let path = path.to_string_lossy().to_string();
    let metadata = ScannedFile {
        title: tag
            .as_ref()
            .and_then(|t| t.title().map(|s| s.to_string()))
            .unwrap_or_else(|| path.split("/").last().unwrap_or_default().to_string()),
        artist: tag
            .as_ref()
            .and_then(|t| t.artist().map(|s| s.to_string()))
            .unwrap_or_else(String::new),
        album: tag
            .as_ref()
            .and_then(|t| t.album().map(|s| s.to_string()))
            .unwrap_or_else(String::new),
        year: tag
            .as_ref()
            .and_then(|t| t.year().map(|y| y as u16))
            .unwrap_or(0),
        genre: tag
            .as_ref()
            .and_then(|t| t.genre().map(|s| s.to_string()))
            .unwrap_or_else(String::new),
        duration: duration.as_secs() as u64,
        artwork: cover_art,
        path,
        kind: ScannedKind::LocalFile,
    };

    Ok(metadata)
}

pub fn scan_folder(folder: &PathBuf) -> Vec<ScannedFile> {
    // List of supported audio file extensions
    let supported_exts = ["mp3", "flac", "ogg", "wav", "m4a", "aac", "aiff"];

    // Collect all file paths with supported extensions
    let files: Vec<PathBuf> = WalkDir::new(folder)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| supported_exts.contains(&ext.to_lowercase().as_str()))
                .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    // Process files in parallel
    files
        .par_iter()
        .filter_map(|path| scan_file(path).ok())
        .collect()
}
