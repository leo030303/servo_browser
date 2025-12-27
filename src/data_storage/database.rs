pub fn init_db(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS browser_history (
            id   INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            url TEXT NOT NULL,
            time_accessed TEXT NOT NULL
        )",
        (),
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS bookmarks (
            id   INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            url TEXT NOT NULL,
            time_modified TEXT NOT NULL
        )",
        (),
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS open_tabs (
            id   INTEGER PRIMARY KEY,
            url TEXT NOT NULL
        )",
        (),
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS download_history (
            id   INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            url TEXT NOT NULL,
            save_path TEXT NOT NULL,
            file_size_in_bytes INTEGER NOT NULL,
            time_downloaded TEXT NOT NULL
        )",
        (),
    )?;
    Ok(())
}

// pub fn load_browser_data() -> BrowserData {
//     let conn = Connection::open(default_config_dir().join("browser_data.db")).unwrap();
//     let browser_history = conn
//         .prepare("SELECT id, title, url, time_accessed FROM browser_history")
//         .unwrap()
//         .query_map([], |row| {
//             Ok(HistoryEntry {
//                 id: row.get(0).unwrap(),
//                 title: row.get(1).unwrap(),
//                 url: row.get(2).unwrap(),
//                 time_accessed: row.get(3).unwrap(),
//             })
//         })
//         .unwrap()
//         .map(|item| item.unwrap())
//         .collect();
//     let open_tabs = conn
//         .prepare("SELECT id, title, url FROM open_tabs")
//         .unwrap()
//         .query_map([], |row| {
//             Ok(OpenTab {
//                 id: row.get(0).unwrap(),
//                 title: row.get(1).unwrap(),
//                 url: row.get(2).unwrap(),
//             })
//         })
//         .unwrap()
//         .map(|item| item.unwrap())
//         .collect();
//     let download_history = conn
//         .prepare("SELECT id, title, url, save_path, file_size_in_bytes, time_downloaded FROM download_history")
//         .unwrap()
//         .query_map([], |row| {
//             Ok(DownloadEntry {
//                 id: row.get(0).unwrap(),
//                 title: row.get(1).unwrap(),
//                 url: row.get(2).unwrap(),
//                 save_path: PathBuf::from(row.get::<usize, String>(3).unwrap()),
//                 file_size_in_bytes: row.get(4).unwrap(),
//                 time_downloaded: row.get(5).unwrap(),
//             })
//         })
//         .unwrap()
//         .map(|item| item.unwrap())
//         .collect();
//     let bookmarks = conn
//         .prepare("SELECT id, title, url, time_modified FROM bookmarks")
//         .unwrap()
//         .query_map([], |row| {
//             Ok(BookmarkEntry {
//                 id: row.get(0).unwrap(),
//                 title: row.get(1).unwrap(),
//                 url: row.get(2).unwrap(),
//                 time_modified: row.get(3).unwrap(),
//             })
//         })
//         .unwrap()
//         .map(|item| item.unwrap())
//         .collect();
//     BrowserData {
//         browser_history,
//         open_tabs,
//         download_history,
//         bookmarks,
//     }
// }

// pub fn save_browser_data(browser_data: &BrowserData) {
//     let conn = Connection::open(default_config_dir().join("browser_data.db")).unwrap();
//     browser_data.browser_history.iter().for_each(|item| {
//         conn.execute(
//             "INSERT INTO browser_history (id, title, url, time_accessed) VALUES (?1, ?2, ?3, ?4)",
//             (&item.id, &item.title, &item.url, &item.time_accessed),
//         )
//         .unwrap();
//     });
//     browser_data.open_tabs.iter().for_each(|item| {
//         conn.execute(
//             "INSERT INTO open_tabs (id, title, url) VALUES (?1, ?2, ?3)",
//             (&item.id, &item.title, &item.url),
//         )
//         .unwrap();
//     });
//     browser_data.download_history.iter().for_each(|item| {
//         conn.execute(
//             "INSERT INTO download_history (id, title, url, save_path, file_size_in_bytes, time_downloaded) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
//             (&item.id, &item.title, &item.url, &item.save_path.to_str().unwrap(), &item.file_size_in_bytes, &item.time_downloaded),
//         )
//         .unwrap();
//     });
//     browser_data.bookmarks.iter().for_each(|item| {
//         conn.execute(
//             "INSERT INTO bookmarks (id, title, url, time_modified) VALUES (?1, ?2, ?3, ?4)",
//             (&item.id, &item.title, &item.url, &item.time_modified),
//         )
//         .unwrap();
//     });
// }
