use rusqlite::Connection;
use chrono::{DateTime, Utc};

use crate::error::AppError;
use crate::models::{HistoryEntry, Platform, TaskStatus};

/// Persist a completed/failed/cancelled task to history.
pub fn save_history_entry(conn: &Connection, entry: &HistoryEntry) -> Result<(), AppError> {
    conn.execute(
        "INSERT OR REPLACE INTO history (id, url, platform, author, title, status, save_path, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            entry.id,
            entry.url,
            serde_json::to_string(&entry.platform).unwrap_or_default().trim_matches('"'),
            entry.author,
            entry.title,
            serde_json::to_string(&entry.status).unwrap_or_default().trim_matches('"'),
            entry.save_path,
            entry.created_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Query history with optional search and status filter.
pub fn get_history(
    conn: &Connection,
    search: Option<&str>,
    status_filter: Option<&str>,
) -> Result<Vec<HistoryEntry>, AppError> {
    let mut sql = "SELECT id, url, platform, author, title, status, save_path, created_at FROM history WHERE 1=1".to_string();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(search_term) = search {
        if !search_term.is_empty() {
            sql.push_str(" AND (title LIKE ?1 OR author LIKE ?1 OR url LIKE ?1)");
            params.push(Box::new(format!("%{}%", search_term)));
        }
    }

    if let Some(status) = status_filter {
        if !status.is_empty() {
            let param_idx = params.len() + 1;
            sql.push_str(&format!(" AND status = ?{}", param_idx));
            params.push(Box::new(status.to_string()));
        }
    }

    sql.push_str(" ORDER BY created_at DESC");

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        let platform_str: String = row.get(2)?;
        let status_str: String = row.get(5)?;
        let created_at_str: String = row.get(7)?;

        Ok(HistoryEntry {
            id: row.get(0)?,
            url: row.get(1)?,
            platform: parse_platform(&platform_str),
            author: row.get(3)?,
            title: row.get(4)?,
            status: parse_status(&status_str),
            save_path: row.get(6)?,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    })?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }

    Ok(entries)
}

/// Clear all history entries.
pub fn clear_history(conn: &Connection) -> Result<(), AppError> {
    conn.execute("DELETE FROM history", [])?;
    Ok(())
}

fn parse_platform(s: &str) -> Platform {
    match s {
        "douyin" => Platform::Douyin,
        "xiaohongshu" => Platform::Xiaohongshu,
        _ => Platform::Douyin, // Default fallback
    }
}

fn parse_status(s: &str) -> TaskStatus {
    match s {
        "completed" => TaskStatus::Completed,
        "failed" => TaskStatus::Failed,
        "cancelled" => TaskStatus::Cancelled,
        _ => TaskStatus::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE history (
                id TEXT PRIMARY KEY,
                url TEXT NOT NULL,
                platform TEXT NOT NULL,
                author TEXT,
                title TEXT,
                status TEXT NOT NULL,
                save_path TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX idx_history_status ON history(status);",
        )
        .unwrap();
        conn
    }

    fn make_entry(id: &str, status: TaskStatus) -> HistoryEntry {
        HistoryEntry {
            id: id.to_string(),
            url: format!("https://example.com/{}", id),
            platform: Platform::Douyin,
            author: Some("test_author".to_string()),
            title: Some("test_title".to_string()),
            status,
            save_path: Some("/downloads/test.mp4".to_string()),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_save_and_get_history() {
        let conn = setup_db();
        let entry = make_entry("1", TaskStatus::Completed);
        save_history_entry(&conn, &entry).unwrap();

        let results = get_history(&conn, None, None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "1");
    }

    #[test]
    fn test_search_history() {
        let conn = setup_db();
        save_history_entry(&conn, &make_entry("1", TaskStatus::Completed)).unwrap();

        let mut entry2 = make_entry("2", TaskStatus::Failed);
        entry2.title = Some("unique_title".to_string());
        save_history_entry(&conn, &entry2).unwrap();

        let results = get_history(&conn, Some("unique"), None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "2");
    }

    #[test]
    fn test_filter_by_status() {
        let conn = setup_db();
        save_history_entry(&conn, &make_entry("1", TaskStatus::Completed)).unwrap();
        save_history_entry(&conn, &make_entry("2", TaskStatus::Failed)).unwrap();
        save_history_entry(&conn, &make_entry("3", TaskStatus::Cancelled)).unwrap();

        let results = get_history(&conn, None, Some("completed")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "1");
    }

    #[test]
    fn test_clear_history() {
        let conn = setup_db();
        save_history_entry(&conn, &make_entry("1", TaskStatus::Completed)).unwrap();
        save_history_entry(&conn, &make_entry("2", TaskStatus::Failed)).unwrap();

        clear_history(&conn).unwrap();
        let results = get_history(&conn, None, None).unwrap();
        assert!(results.is_empty());
    }
}
