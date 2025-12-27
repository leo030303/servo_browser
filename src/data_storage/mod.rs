use database::init_db;
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

    pub fn save_open_tabs(&self, open_tabs: &[String]) {
        self.connection
            .execute("DELETE FROM open_tabs;", ())
            .unwrap();
        open_tabs.iter().for_each(|url| {
            self.connection
                .execute("INSERT INTO open_tabs (url) VALUES (?1)", (&url,))
                .unwrap();
        });
    }

    pub fn load_open_tabs(&self) -> Vec<OpenTab> {
        self.connection
            .prepare("SELECT id, url FROM open_tabs")
            .unwrap()
            .query_map([], |row| {
                Ok(OpenTab {
                    id: row.get(0).unwrap(),
                    url: row.get(1).unwrap(),
                })
            })
            .unwrap()
            .map(|item| item.unwrap())
            .collect()
    }
}
