use bookmarks::BookmarkEntry;
use database::init_db;
use downloads::DownloadEntry;
use history::HistoryEntry;
use tabs::OpenTab;

use crate::prefs::default_config_dir;

pub mod bookmarks;
pub mod database;
pub mod downloads;
pub mod history;
pub mod tabs;

#[derive(Debug)]
pub struct BrowserDataConnection {
    connection: rusqlite::Connection,
}

impl BrowserDataConnection {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let connection =
            rusqlite::Connection::open(default_config_dir().join("browser_data.db")).unwrap();
        init_db(&connection).unwrap();
        Self { connection }
    }
    pub fn add_to_browser_history(&self, page_title: String, page_url: String) {
        self.connection
            .execute(
                "INSERT INTO browser_history (title, url, time_accessed) VALUES (?1, ?2, ?3)",
                (&page_title, &page_url, &chrono::Utc::now().naive_utc()),
            )
            .unwrap();
    }

    pub fn get_browser_history(&self) -> Vec<HistoryEntry> {
        self.connection
            .prepare("SELECT id, title, url, time_accessed FROM browser_history")
            .unwrap()
            .query_map([], |row| {
                Ok(HistoryEntry {
                    id: row.get(0).unwrap(),
                    title: row.get(1).unwrap(),
                    url: row.get(2).unwrap(),
                    time_accessed: row.get(3).unwrap(),
                })
            })
            .unwrap()
            .map(|item| item.unwrap())
            .collect()
    }
}
