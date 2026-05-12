use rusqlite::Connection;
use crate::error::AppError;
use crate::models::AppSettings;

/// Supported filename template tokens.
const VALID_TOKENS: &[&str] = &[
    "{platform}", "{author}", "{title}", "{date}", "{index}", "{ext}",
];

/// Read all settings from the database.
pub fn get_settings(conn: &Connection) -> Result<AppSettings, AppError> {
    let mut settings = AppSettings::default();

    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for row in rows {
        let (key, value) = row?;
        match key.as_str() {
            "download_dir" => settings.download_dir = value,
            "max_concurrency" => {
                settings.max_concurrency = value.parse().unwrap_or(3).clamp(1, 8);
            }
            "filename_template" => settings.filename_template = value,
            "auto_clipboard" => settings.auto_clipboard = value == "true",
            "keep_history" => settings.keep_history = value == "true",
            "debug_log" => settings.debug_log = value == "true",
            "theme" => settings.theme = value,
            "language" => settings.language = value,
            _ => {}
        }
    }

    Ok(settings)
}

/// Update settings atomically using a transaction.
pub fn update_settings(conn: &Connection, settings: &AppSettings) -> Result<(), AppError> {
    // Validate max_concurrency
    if settings.max_concurrency < 1 || settings.max_concurrency > 8 {
        return Err(AppError::InvalidInput {
            message: "max_concurrency must be between 1 and 8".to_string(),
        });
    }

    // Validate filename_template
    validate_template(&settings.filename_template)?;

    let tx = conn.unchecked_transaction()?;

    let pairs = vec![
        ("download_dir", settings.download_dir.clone()),
        ("max_concurrency", settings.max_concurrency.to_string()),
        ("filename_template", settings.filename_template.clone()),
        ("auto_clipboard", settings.auto_clipboard.to_string()),
        ("keep_history", settings.keep_history.to_string()),
        ("debug_log", settings.debug_log.to_string()),
        ("theme", settings.theme.clone()),
        ("language", settings.language.clone()),
    ];

    for (key, value) in pairs {
        tx.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )?;
    }

    tx.commit()?;
    Ok(())
}

/// Validate a filename template against supported tokens.
fn validate_template(template: &str) -> Result<(), AppError> {
    // Find all tokens in the template (anything between { and })
    let mut i = 0;
    let chars: Vec<char> = template.chars().collect();

    while i < chars.len() {
        if chars[i] == '{' {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != '}' {
                i += 1;
            }
            if i < chars.len() {
                let token: String = chars[start..=i].iter().collect();
                if !VALID_TOKENS.contains(&token.as_str()) {
                    return Err(AppError::InvalidInput {
                        message: format!("Unknown template token: {}", token),
                    });
                }
            }
        }
        i += 1;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_template_valid() {
        assert!(validate_template("{platform}/{author}/{date}/{title}_{index}.{ext}").is_ok());
        assert!(validate_template("{title}.{ext}").is_ok());
        assert!(validate_template("plain_text").is_ok());
    }

    #[test]
    fn test_validate_template_invalid() {
        assert!(validate_template("{unknown_token}").is_err());
        assert!(validate_template("{platform}/{invalid}").is_err());
    }

    #[test]
    fn test_settings_roundtrip() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )
        .unwrap();

        let settings = AppSettings {
            download_dir: "/tmp/downloads".to_string(),
            max_concurrency: 5,
            filename_template: "{title}.{ext}".to_string(),
            auto_clipboard: true,
            keep_history: false,
            debug_log: true,
            theme: "dark".to_string(),
            language: "en-US".to_string(),
        };

        update_settings(&conn, &settings).unwrap();
        let loaded = get_settings(&conn).unwrap();

        assert_eq!(loaded.download_dir, "/tmp/downloads");
        assert_eq!(loaded.max_concurrency, 5);
        assert_eq!(loaded.auto_clipboard, true);
        assert_eq!(loaded.keep_history, false);
        assert_eq!(loaded.theme, "dark");
        assert_eq!(loaded.language, "en-US");
    }

    #[test]
    fn test_settings_validation_rejects_invalid_concurrency() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )
        .unwrap();

        let mut settings = AppSettings::default();
        settings.max_concurrency = 10; // Invalid: > 8
        assert!(update_settings(&conn, &settings).is_err());

        settings.max_concurrency = 0; // Invalid: < 1
        assert!(update_settings(&conn, &settings).is_err());
    }
}
