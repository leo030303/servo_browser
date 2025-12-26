#[derive(Debug)]
pub struct BookmarkEntry {
    pub id: i32,
    pub title: String,
    pub url: String,
    pub time_modified: chrono::NaiveDateTime,
}
