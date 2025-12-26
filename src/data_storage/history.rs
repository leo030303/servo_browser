#[derive(Debug)]
pub struct HistoryEntry {
    pub id: i32,
    pub title: String,
    pub url: String,
    pub time_accessed: chrono::NaiveDateTime,
}
