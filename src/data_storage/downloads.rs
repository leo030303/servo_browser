use std::path::PathBuf;

#[derive(Debug)]
pub struct DownloadEntry {
    pub id: i32,
    pub title: String,
    pub url: String,
    pub save_path: PathBuf,
    pub file_size_in_bytes: u32,
    pub time_downloaded: chrono::NaiveDateTime,
}
